//! What the runtime measured of the room, which is `XR_META_environment_depth` driven by hand.
//!
//! It is the extension whose direction is backwards. Every other swapchain goes from the app to the
//! compositor; this one goes the other way - the runtime writes a depth map into it and the app reads it - so
//! it is not an `XrSwapchain` at all: an image is taken for a frame with `xrAcquireEnvironmentDepthImageMETA`
//! and given back by the frame ending.
//!
//! The cost is a rule with teeth: at most one acquire per frame, and only between `xrBeginFrame` and
//! `xrEndFrame`. The core's [`wxr::Session::environment_depth`] is asked once *per view*, and asking twice is
//! `XR_ERROR_LIMIT_REACHED` - so the acquire happens on the first ask of a frame and both views read the one
//! measurement. That is why the provider and its swapchain live in [`EnvironmentDepth`] and the measurement
//! is carried separately: one belongs to the session and one belongs to the frame.
//!
//! **What OpenXR does not give:** a D16 buffer of *window* depth and its planes, where WebXR hands back a
//! scale and a transform. Neither answers in the other's terms - see [`EnvironmentDepth::info`].

use super::*;

/// The runtime's depth map, as the renderer names one.
///
/// This is [`wxr::Session::EnvironmentDepth`] for this backend, and it carries the format for the same reason
/// [`wxr::Session::Image`] does not have to: the runtime chose it (`VK_FORMAT_D16_UNORM`, always, and the
/// app is not asked) and the core's vocabulary has no depth formats in it at all - [`wxr::ColorFormat`] is
/// colour. So the number travels, and the importer is the one that names it.
#[derive(Clone, Copy, Debug)]
pub struct EnvironmentDepthImage {
    /// One of the depth swapchain's images, as the runtime names it: a `VkImage`, an integer here.
    pub(crate) image: u64,
    /// What it is, as Vulkan names it.
    pub(crate) format: u32,
    /// The size of one layer; there are two, one per eye, as array layers of this one image.
    pub(crate) extent: wxr::Extent2d,
}

/// The depth provider, its swapchain, and where its images are.
pub(super) struct EnvironmentDepth {
    /// The session, kept for its `Arc`: the two handles below are the session's children, and a child
    /// destroyed after its parent is a call into a handle the runtime has already freed. Holding the session
    /// is what makes this struct's own `Drop` safe to make at all - see [`Drop for EnvironmentDepth`].
    #[allow(dead_code, reason = "held for its lifetime, not for a call")]
    session: xr::Session<xr::Vulkan>,
    table: xr::raw::EnvironmentDepthMETA,
    provider: openxr_sys::EnvironmentDepthProviderMETA,
    swapchain: openxr_sys::EnvironmentDepthSwapchainMETA,
    /// The images, in the order the runtime enumerated them. Constant for the life of the swapchain, which
    /// is what the spec promises and what makes it safe to hand one of them to a renderer.
    images: Vec<u64>,
    /// What one image is: the format the extension mandates, and the size the runtime chose.
    depth: EnvironmentDepthImage,
    /// Whether the provider is running. The extension refuses a start of one already started, and a stop of
    /// one already stopped, so this is what keeps a caller that says the same thing every frame from failing.
    started: bool,
    /// Whether the caller wants depth. A pause clears it and an ask does not set it again, so only
    /// [`wxr::Session::set_depth_active`] resumes a paused provider - that call is a state, not a hint.
    wanted: bool,
    /// Whether the near and far planes have been said out loud yet, which is once and not every frame.
    logged: bool,
}

impl EnvironmentDepth {
    /// Make the provider and its swapchain, or say this runtime cannot measure the room.
    ///
    /// Made and not started: [`Self::start`] runs on the first ask or when the caller says so, because
    /// producing depth for an app that never looks is holding a camera for nothing.
    pub(super) fn new(session: &xr::Session<xr::Vulkan>, enabled: bool) -> Option<Self> {
        if !enabled {
            log::info!("wxr-openxr: no depth: this runtime has no XR_META_environment_depth");
            return None;
        }
        let Some(table) = session.instance().exts().meta_environment_depth.as_ref() else {
            log::info!("wxr-openxr: no depth: the extension was enabled but did not load");
            return None;
        };

        let provider_info = openxr_sys::EnvironmentDepthProviderCreateInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_PROVIDER_CREATE_INFO_META,
            next: std::ptr::null(),
            // The extension defines no flag bits: "There are currently no flag bits defined. This is
            // reserved for future use."
            create_flags: openxr_sys::EnvironmentDepthProviderCreateFlagsMETA::EMPTY,
        };
        let mut provider = null_provider();
        // SAFETY: the session and the table are the runtime's own and outlive the call, and `provider` is
        // the out-parameter the runtime writes the handle into.
        let result = unsafe {
            (table.create_environment_depth_provider)(
                session.as_raw(),
                &provider_info,
                &mut provider,
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: making a provider: {result:?}");
            return None;
        }

        let swapchain_info = openxr_sys::EnvironmentDepthSwapchainCreateInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_SWAPCHAIN_CREATE_INFO_META,
            next: std::ptr::null(),
            create_flags: openxr_sys::EnvironmentDepthSwapchainCreateFlagsMETA::EMPTY,
        };
        let mut swapchain = null_swapchain();
        // SAFETY: the provider was just made and is live, and the out-parameter is as above.
        let result = unsafe {
            (table.create_environment_depth_swapchain)(provider, &swapchain_info, &mut swapchain)
        };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: making its swapchain: {result:?}");
            undo(table, provider, None);
            return None;
        }

        // The size is the runtime's to choose and is constant for the swapchain's life, so it is asked once
        // here rather than every frame. Asking is how the answer exists at all - the extension hands the
        // size back in exactly this structure and nowhere else.
        let mut state = openxr_sys::EnvironmentDepthSwapchainStateMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_SWAPCHAIN_STATE_META,
            next: std::ptr::null_mut(),
            width: 0,
            height: 0,
        };
        // SAFETY: the swapchain is live and `state` is an out-structure with its header filled in.
        let result =
            unsafe { (table.get_environment_depth_swapchain_state)(swapchain, &mut state) };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: asking how big it is: {result:?}");
            undo(table, provider, Some(swapchain));
            return None;
        }

        // The images, by the same two-call idiom every OpenXR list has.
        let mut count = 0u32;
        // SAFETY: a zero capacity with a null array is how an app asks for a count, and `count` is the
        // out-parameter it goes into.
        let result = unsafe {
            (table.enumerate_environment_depth_swapchain_images)(
                swapchain,
                0,
                &mut count,
                std::ptr::null_mut(),
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: asking how many images there are: {result:?}");
            undo(table, provider, Some(swapchain));
            return None;
        }
        if count == 0 || state.width == 0 || state.height == 0 {
            log::warn!("wxr-openxr: no depth: the swapchain has no usable image");
            undo(table, provider, Some(swapchain));
            return None;
        }
        // Each element is a graphics-API structure with a `type` and a `next`, so each is built rather than
        // left as whatever memory it is - which is a rule the runtime checks.
        let mut images: Vec<openxr_sys::SwapchainImageVulkanKHR> = (0..count)
            .map(|_| openxr_sys::SwapchainImageVulkanKHR {
                ty: openxr_sys::StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR,
                next: std::ptr::null_mut(),
                image: 0,
            })
            .collect();
        // SAFETY: the buffer holds `count` elements, each with the type and null chain the runtime is told
        // to expect, and it is cast to the base header the call names - which has the same leading two
        // fields, so the cast widens the type and not the layout.
        let result = unsafe {
            (table.enumerate_environment_depth_swapchain_images)(
                swapchain,
                count,
                &mut count,
                images.as_mut_ptr().cast(),
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: reading the images: {result:?}");
            undo(table, provider, Some(swapchain));
            return None;
        }

        log::info!(
            "wxr-openxr: the runtime measures depth at {}x{} px, {} images",
            state.width,
            state.height,
            images.len()
        );
        Some(Self {
            session: session.clone(),
            table: *table,
            provider,
            swapchain,
            images: images.into_iter().map(|image| image.image).collect(),
            depth: EnvironmentDepthImage {
                image: 0,
                format: ash::vk::Format::D16_UNORM.as_raw() as u32,
                extent: wxr::Extent2d::new(state.width, state.height),
            },
            started: false,
            wanted: true,
            logged: false,
        })
    }

    /// Start or stop the runtime measuring, which is `xrStartEnvironmentDepthProviderMETA` and its stop.
    ///
    /// A provider already in the asked state is answered without a call, because the extension refuses both
    /// of those calls on a provider already in that state - so a caller that says "on" every frame must not
    /// be a caller that fails on the second one.
    pub(super) fn set_active(&mut self, active: bool) -> Result<(), wxr::Error> {
        self.wanted = active;
        if self.started == active {
            return Ok(());
        }
        // SAFETY: the provider is live.
        let result = unsafe {
            if active {
                (self.table.start_environment_depth_provider)(self.provider)
            } else {
                (self.table.stop_environment_depth_provider)(self.provider)
            }
        };
        if result != openxr_sys::Result::SUCCESS {
            return Err(wxr::Error::Rejected(format!("{result:?}").into()));
        }
        self.started = active;
        Ok(())
    }

    /// The first ask of a frame starts the provider, when the caller has not paused it.
    fn start(&mut self) -> bool {
        match self.set_active(true) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("wxr-openxr: no depth: starting the provider: {error:?}");
                false
            }
        }
    }

    /// This frame's measurement, or nothing if the runtime has none yet.
    ///
    /// "Nothing" is a normal answer: a runtime "must return XR_ENVIRONMENT_DEPTH_NOT_AVAILABLE_META if no
    /// depth frame is available yet", which is a success code and so is matched by name, not as a failure.
    pub(super) fn acquire(
        &mut self,
        base_space: openxr_sys::Space,
        time: xr::Time,
    ) -> Option<MeasuredDepth> {
        let acquire_info = openxr_sys::EnvironmentDepthImageAcquireInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_IMAGE_ACQUIRE_INFO_META,
            next: std::ptr::null(),
            // The space the frame's views were located in: what matches the frame's picture is the answer,
            // and the first space the app made was never necessarily that.
            space: base_space,
            display_time: time,
        };
        let mut image = openxr_sys::EnvironmentDepthImageMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_IMAGE_META,
            next: std::ptr::null(),
            swapchain_index: 0,
            near_z: 0.0,
            far_z: 0.0,
            // Two views and never another number: the extension makes the image an array of exactly two.
            views: [depth_view(); 2],
        };
        // SAFETY: the provider is live and started, the frame is between its begin and its end, and `image`
        // is an out-structure with its header filled in and both view slots already built.
        let result = unsafe {
            (self.table.acquire_environment_depth_image)(self.provider, &acquire_info, &mut image)
        };
        if result == openxr_sys::Result::ENVIRONMENT_DEPTH_NOT_AVAILABLE_META {
            return None;
        }
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: taking this frame's depth: {result:?}");
            return None;
        }
        let index = image.swapchain_index as usize;
        // Said once: the planes are the same every frame, and a log line per frame is not information.
        if !self.logged {
            self.logged = true;
            log::info!(
                "wxr-openxr: the room's depth spans {}..{} m, {}x{} px",
                image.near_z,
                image.far_z,
                self.depth.extent.width,
                self.depth.extent.height
            );
        }
        let Some(source) = self.images.get(index) else {
            log::debug!(
                "wxr-openxr: the runtime gave depth image {index}, which is past its own {}",
                self.images.len()
            );
            return None;
        };
        Some(MeasuredDepth {
            image: EnvironmentDepthImage {
                image: *source,
                ..self.depth
            },
            info: self.info(image.near_z, image.far_z),
        })
    }

    /// What this frame's buffer means, in the core's terms, from the two planes the runtime reported with it.
    ///
    /// Window depth *is* a hyperbolic function of distance, so there is no `raw_value_to_meters` scalar that
    /// could describe this buffer - which is why it answers in planes rather than in WebXR's scale.
    /// `XrEnvironmentDepthImageMETA` reports `nearZ` and `farZ` with every acquire, and they are put where
    /// [`wxr::DepthEncoding::Window`] expects them. That type's own note carries the one trap, that
    /// `far_z < near_z` is an infinite frustum rather than a reversed one.
    ///
    /// **The assumption:** `XR_KHR_composition_layer_depth` states the description as a projection **matrix**
    /// over four numbers - `minDepth`/`maxDepth`/`nearZ`/`farZ` - and this extension reports only the last two.
    /// The first two are 0 and 1 for any buffer whose near plane sits at zero, which a depth attachment does
    /// unless something clamped it, and this extension cannot say otherwise. That is the pair
    /// `depth_submission` uses for the app's own buffer, and it is the one thing here no machine in this
    /// workspace can check: nothing that runs on it has this extension.
    ///
    /// `norm_depth_buffer_from_norm_view` is identity: the buffer was rendered from this eye's pose with this
    /// eye's field of view and covers the whole texture, so normalized buffer `x` and `y` *are* normalized view
    /// ones. WebXR's transform carries a projection along `z` beyond that, which a rigid pose cannot carry -
    /// which is what the encoding above is for. The eye's field of view and pose are not reported either: the
    /// core's [`wxr::DepthInfo`] has no slot, and a renderer that wants to un-project asks the frame's views,
    /// which are the same two.
    ///
    /// A `near_z` the runtime never filled in is not a plane, so it is [`wxr::DepthEncoding::Unknown`] rather
    /// than a frustum at zero - the one case where this still answers with nothing.
    fn info(&self, near_z: f32, far_z: f32) -> wxr::DepthInfo {
        let encoding = if near_z > 0.0 {
            wxr::DepthEncoding::Window { near_z, far_z }
        } else {
            wxr::DepthEncoding::Unknown
        };
        wxr::DepthInfo {
            size: self.depth.extent,
            encoding,
            norm_depth_buffer_from_norm_view: wxr::Pose::IDENTITY,
        }
    }
}

impl OpenXrSession {
    /// The room, measured, for the view asked for.
    ///
    /// Both views answer the same thing, and that is not a shortcut: the measurement is one image with two
    /// array layers, and what would differ per eye - field of view and pose - has no slot in
    /// [`wxr::DepthInfo`]. A renderer that wants the eye asks the frame's views, which are the same two.
    pub(super) fn environment_depth_impl(
        &mut self,
        view: usize,
    ) -> Option<(&EnvironmentDepthImage, wxr::DepthInfo)> {
        // Only inside a frame: the acquire is a call between `xrBeginFrame` and `xrEndFrame`, and the
        // image belongs to that pair. `end` drops `held`, so a caller between frames takes nothing.
        self.held?;
        // The first ask of a frame is the one that tries to take the image: the runtime allows one acquire
        // between a frame's begin and its end, and a second is `XR_ERROR_LIMIT_REACHED` - even when the first
        // answered that no depth was ready. A view this frame did not locate is nothing to measure for,
        // including either eye before `views` has filled the list: that list is what says a view exists.
        if view >= self.located.len() {
            return None;
        }
        if !self.depth_asked {
            let depth = self.depth.as_mut()?;
            // A caller that paused depth has said what it wants, and asking for a measurement does not undo
            // it: only `set_depth_active(true)` does.
            if !depth.wanted || !depth.start() {
                return None;
            }
            // Said only once the runtime is asked: a paused ask did not use the frame's one acquire.
            self.depth_asked = true;
            // The space the projection layer is drawn in, which is the one the views were located in - the
            // first one the app made was never necessarily that. Taken as the raw handle because the depth
            // provider and this backend's list of spaces are separate things.
            let base = self
                .view_space
                .and_then(|id| self.spaces.get(id))
                .or_else(|| self.spaces.first())?
                .as_raw();
            self.measured = depth.acquire(base, self.predicted);
        }
        let measured = self.measured.as_ref()?;
        Some((&measured.image, measured.info))
    }

    /// Start or stop the runtime measuring the room, which is WebXR's `resumeDepthSensing` and its pause.
    ///
    /// No provider is no depth to pause, which is `Unsupported`, and a request matching the state already
    /// reported is a no-op, so a caller may say what it wants every frame. A stop drops this frame's
    /// measurement too.
    pub(super) fn set_depth_active_impl(&mut self, active: bool) -> Result<(), wxr::Error> {
        let Some(depth) = self.depth.as_mut() else {
            return Err(wxr::Error::Unsupported("depth sensing".into()));
        };
        depth.set_active(active)?;
        if !active {
            self.measured = None;
        }
        Ok(())
    }

    /// What this session's depth sensing is, once it has a way to measure.
    pub(super) fn depth_sensing_impl(&self) -> Option<wxr::DepthSensing> {
        let depth = self.depth.as_ref()?;
        Some(wxr::DepthSensing {
            // A GPU texture the scene samples where it lies, which is the whole of what this extension is: a
            // swapchain image rather than a buffer to read.
            usage: wxr::DepthUsage::GpuOptimized,
            // `VK_FORMAT_D16_UNORM`, which is sixteen bits of unsigned normalized value.
            format: wxr::DepthFormat::UnsignedShort,
            // The runtime does not say whether what it produces is what the sensor measured or a surface
            // fitted to it, and this extension has no way to ask.
            ty: None,
            // And this one is knowable, and it is a fact about *now*: the provider is made at connect and
            // started by the first ask or `set_depth_active`, so this says which of the two it is.
            active: Some(depth.started),
        })
    }
}

/// What this backend holds for a frame's measurement: the image, and what its values mean.
pub(super) struct MeasuredDepth {
    pub(super) image: EnvironmentDepthImage,
    pub(super) info: wxr::DepthInfo,
}

/// One eye's metadata slot, built the way the runtime is told to expect.
fn depth_view() -> openxr_sys::EnvironmentDepthImageViewMETA {
    openxr_sys::EnvironmentDepthImageViewMETA {
        ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_IMAGE_VIEW_META,
        next: std::ptr::null(),
        fov: openxr_sys::Fovf {
            angle_left: 0.0,
            angle_right: 0.0,
            angle_up: 0.0,
            angle_down: 0.0,
        },
        pose: xr::Posef::IDENTITY,
    }
}

/// A null depth provider, which is what the out-parameter of the create call begins as.
fn null_provider() -> openxr_sys::EnvironmentDepthProviderMETA {
    // SAFETY: this is the null handle, and nothing dereferences it until the runtime has written one.
    unsafe { std::mem::zeroed() }
}

/// A null depth swapchain, which is what the out-parameter of the create call begins as.
fn null_swapchain() -> openxr_sys::EnvironmentDepthSwapchainMETA {
    // SAFETY: this is the null handle, and nothing dereferences it until the runtime has written one.
    unsafe { std::mem::zeroed() }
}

/// Undo what had been made when a later step said no.
///
/// The same two calls in the order [`Drop for EnvironmentDepth`] gives them, with the stop left out because
/// nothing was started. `None` is the return before a swapchain existed.
fn undo(
    table: &xr::raw::EnvironmentDepthMETA,
    provider: openxr_sys::EnvironmentDepthProviderMETA,
    swapchain: Option<openxr_sys::EnvironmentDepthSwapchainMETA>,
) {
    // SAFETY: both handles were made above, are being destroyed exactly once, and the swapchain goes before
    // the provider because the spec says a provider must not be destroyed until its swapchains are.
    unsafe {
        if let Some(swapchain) = swapchain {
            (table.destroy_environment_depth_swapchain)(swapchain);
        }
        (table.destroy_environment_depth_provider)(provider);
    }
}

impl Drop for EnvironmentDepth {
    fn drop(&mut self) {
        // The spec's order: a provider "must not be destroyed until all of its swapchains have been", and
        // stopping first too - a provider still producing into a swapchain that is about to go is the one
        // thing here that could race with the runtime rather than merely leak.
        //
        // A failure is nothing to do about: the handles are about to be dropped either way, so what is said
        // is only that it did not go smoothly.
        let (stopped, swapchain_gone, provider_gone) = unsafe {
            let stopped = if self.started {
                (self.table.stop_environment_depth_provider)(self.provider)
            } else {
                openxr_sys::Result::SUCCESS
            };
            let swapchain_gone = (self.table.destroy_environment_depth_swapchain)(self.swapchain);
            let provider_gone = (self.table.destroy_environment_depth_provider)(self.provider);
            (stopped, swapchain_gone, provider_gone)
        };
        if stopped != openxr_sys::Result::SUCCESS
            || swapchain_gone != openxr_sys::Result::SUCCESS
            || provider_gone != openxr_sys::Result::SUCCESS
        {
            log::debug!(
                "wxr-openxr: putting the depth provider back: {stopped:?}, \
                 {swapchain_gone:?}, {provider_gone:?}"
            );
        }
    }
}
