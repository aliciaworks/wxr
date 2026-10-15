//! What the runtime measured of the room, which is `XR_META_environment_depth` driven by hand.
//!
//! This is the extension whose direction is backwards, and that is the whole shape of it. Every swapchain in
//! OpenXR goes from the app to the compositor; this one goes the other way - the runtime writes a depth map
//! into it and the app reads it - and the spec says so in as many words: "This swapchain is different from
//! regular swapchains in that it provides a data channel from the runtime to the application instead of the
//! other way around." So it is not an `XrSwapchain`: there is no `xrAcquireSwapchainImage`, and an image is
//! taken for a frame with `xrAcquireEnvironmentDepthImageMETA` and given back by the frame ending.
//!
//! The cost of that is a rule with teeth: at most one acquire per frame, and only between `xrBeginFrame` and
//! `xrEndFrame`. The core's [`wxr::Session::depth`] is asked once *per view*, and asking twice is
//! `XR_ERROR_LIMIT_REACHED` - so the acquire happens on the first ask of a frame and both views read the one
//! measurement. That is why the provider and its swapchain live in [`EnvironmentDepth`] and the measurement
//! is carried separately: one belongs to the session and one belongs to the frame.
//!
//! **What OpenXR does not give, and this file does not invent.** The extension hands back a D16 buffer of
//! *window* depth plus the near and far planes it was rendered with, and WebXR's depth sensing hands back a
//! `rawValueToMeters` scalar and a `normDepthBufferFromNormView` transform. Neither of the two platforms can
//! answer in the other's terms, and the core says so rather than hiding it: [`wxr::DepthEncoding`] is the two
//! encodings as a choice, so a consumer has to see which one it got before it can turn a number into a
//! distance. The planes this file has always been given and now reports - see [`EnvironmentDepth::info`],
//! which also names the one assumption no machine here can check.
//!
//! The transform still has no counterpart, and that one is not a gap: what WebXR's carries beyond a pose is a
//! projection along `z`, and a rigid pose cannot carry a projection at all. What it would say is instead what
//! the encoding says.

use super::*;

/// The runtime's depth map, as the renderer names one.
///
/// This is [`wxr::Session::Depth`] for this backend, and it carries the format for the same reason
/// [`wxr::Session::Image`] does not have to: the runtime chose it (`VK_FORMAT_D16_UNORM`, always, and the
/// app is not asked) and the core's vocabulary has no depth formats in it at all - [`wxr::ColorFormat`] is
/// colour. So the number travels, and the importer is the one that names it.
#[derive(Clone, Copy, Debug)]
pub struct DepthImage {
    /// One of the depth swapchain's images, as the runtime names it - a `VkImage`, which on this platform is
    /// an integer.
    pub(crate) image: u64,
    /// What it is, as Vulkan names it.
    pub(crate) format: u32,
    /// The size of one layer. There are two, one per eye, and they are array layers of this image rather
    /// than two images - which is why this is one extent and not a list.
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
    depth: DepthImage,
    /// Whether the provider has been started, which the runtime insists happens once and only once.
    started: bool,
    /// Whether the near and far planes have been said out loud yet, which is once and not every frame.
    logged: bool,
}

impl EnvironmentDepth {
    /// Make the provider and its swapchain, or say this runtime cannot measure the room.
    ///
    /// Made and not started: a provider begins producing depth the moment it is started, and a session that
    /// made one for an app that never asks would be holding a camera for nothing. [`Self::start`] runs on
    /// the first ask, which is the first evidence that anyone wants it.
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
        if count == 0 {
            log::warn!("wxr-openxr: no depth: the swapchain holds no images at all");
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
            depth: DepthImage {
                image: 0,
                format: ash::vk::Format::D16_UNORM.as_raw() as u32,
                extent: wxr::Extent2d::new(state.width, state.height),
            },
            started: false,
            logged: false,
        })
    }

    /// Let the runtime start measuring, which it will not do until it is told to.
    ///
    /// Called once, on the first ask, and never reversed: the extension has a stop and this backend does not
    /// use it, because a depth provider that stopped between two frames is a depth map that went missing in
    /// the one place a caller is looking.
    fn start(&mut self) -> bool {
        if self.started {
            return true;
        }
        // SAFETY: the provider is live.
        let result = unsafe { (self.table.start_environment_depth_provider)(self.provider) };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no depth: starting the provider: {result:?}");
            return false;
        }
        self.started = true;
        true
    }

    /// This frame's measurement, or nothing if the runtime has none yet.
    ///
    /// "Nothing" is a real answer and a normal one: a runtime "must return
    /// XR_ENVIRONMENT_DEPTH_NOT_AVAILABLE_META if no depth frame is available yet", which is what a session
    /// that has just started looks like. That result code is a *success* code, which is why it is matched by
    /// name rather than by the call having failed.
    pub(super) fn acquire(
        &mut self,
        base_space: openxr_sys::Space,
        time: xr::Time,
    ) -> Option<MeasuredDepth> {
        let acquire_info = openxr_sys::EnvironmentDepthImageAcquireInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_IMAGE_ACQUIRE_INFO_META,
            next: std::ptr::null(),
            // The reference frame the returned poses come back in: the first space the app made, which is
            // the one the projection layer is drawn in. The core has no slot for a depth query's space, so
            // this is the backend's own choice and it is the one that matches what the frame shows.
            space: base_space,
            display_time: time,
        };
        let mut image = openxr_sys::EnvironmentDepthImageMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_DEPTH_IMAGE_META,
            next: std::ptr::null(),
            swapchain_index: 0,
            near_z: 0.0,
            far_z: 0.0,
            // Two views and never another number: the extension makes the image an array of exactly two, one
            // per eye, and there is no count to ask for.
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
        // The two planes are what turns a sample of this buffer into a distance, and they are now carried
        // into `DepthInfo` rather than dropped - so this line is a statement of what the measurement *is*,
        // once, because it is the same every frame and a log line per frame is not information.
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
            image: DepthImage {
                image: *source,
                ..self.depth
            },
            info: self.info(image.near_z, image.far_z),
        })
    }

    /// What this frame's buffer means, in the core's terms, from the two planes the runtime reported with it.
    ///
    /// This used to answer with `raw_value_to_meters: 0.0` - a number that meant "nothing" because the core
    /// had no slot for anything else - and the reasoning was sound for the wrong reason: window depth *is* a
    /// hyperbolic function of distance, so no scalar exists, and any positive number here would have made a
    /// consumer compare a fraction against a distance and hide the wrong half of the scene. What the reasoning
    /// got wrong was that the two planes are unavailable. They are not: `XrEnvironmentDepthImageMETA` reports
    /// `nearZ` and `farZ` with every acquire, and this backend was reading them, logging them once, and
    /// throwing them away because [`wxr::DepthInfo`] had nowhere to put them.
    ///
    /// So they are put there, as [`wxr::DepthEncoding::Window`], and what it means for a consumer is written
    /// out in that type's own note rather than here - including the one trap, that `far_z < near_z` is an
    /// infinite frustum rather than a reversed one, which the extension says and which nothing in this file
    /// has to decide.
    ///
    /// **What is still missing, and this is now the only gap:** `XR_KHR_composition_layer_depth` states the
    /// whole thing as a projection **matrix** - its own words are "the homogeneous transform from view space
    /// `z` to window space depth" - over four numbers, `minDepth`/`maxDepth`/`nearZ`/`farZ`, and this
    /// extension reports only the last two. The first two are "the window space depths that correspond to the
    /// near and far frustum planes, respectively" - so they are 0 and 1 for any buffer whose near plane sits
    /// at zero, which is what a depth attachment is unless something clamped it, and this extension gives no
    /// way to say otherwise.
    /// [`wxr::DepthEncoding::Window`] is therefore written with 0 and 1 taken as given, and that assumption is
    /// the one thing here that no machine in this workspace can check: nothing that runs on it has this
    /// extension. It is documented at the type so that a reader meets it before believing a distance.
    ///
    /// `norm_depth_buffer_from_norm_view` is the transform from normalized view coordinates into the buffer's.
    /// The buffer is registered to the view - it was rendered from this eye's pose with this eye's field of
    /// view and covers the whole texture - so normalized buffer `x` and `y` *are* normalized view `x` and
    /// `y`, and identity is the exact answer for both. What WebXR's transform carries beyond that is a
    /// projection along `z`, and a rigid pose cannot carry a projection at all; that is what the encoding
    /// above is about.
    ///
    /// The eye's field of view and pose arrive with the measurement and are not reported: the core's
    /// [`wxr::DepthInfo`] has no slot for them, and a renderer that wants to un-project asks the session's
    /// views for theirs, which are the same two.
    ///
    /// And a `near_z` the runtime never filled in is not a plane, so it is [`wxr::DepthEncoding::Unknown`]
    /// rather than a frustum at zero - which is the one case where this still answers with nothing, and the
    /// picture is then the one a session without depth produces.
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
    /// array layers, and what would differ per eye is its field of view and pose - which the core's
    /// [`wxr::DepthInfo`] has nowhere for. A renderer that wants the eye asks the frame's views, which are
    /// the same ones the measurement was taken in.
    pub(super) fn depth_impl(&mut self, view: usize) -> Option<(&DepthImage, wxr::DepthInfo)> {
        // The first ask of a frame is the one that takes the image: the runtime allows one acquire between a
        // frame's begin and its end, and a second is `XR_ERROR_LIMIT_REACHED`. Asking for a view this
        // backend's own views never had is nothing, which is the same answer a caller gets for an index past
        // the end of the frame's list.
        if view >= 2 {
            return None;
        }
        if self.measured.is_none() {
            let depth = self.depth.as_mut()?;
            if !depth.start() {
                return None;
            }
            // The space the projection layer is drawn in, which is the first one the app made. Taken as the
            // raw handle because the depth provider and this backend's list of spaces are separate things.
            let base = self.spaces.first()?.as_raw();
            self.measured = depth.acquire(base, self.predicted);
        }
        let measured = self.measured.as_ref()?;
        Some((&measured.image, measured.info))
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
            // And this one is knowable, and it is a fact about *now* rather than about the session: the
            // provider is made at connect and started at the first ask, so this says which of the two it is.
            active: Some(depth.started),
        })
    }
}

/// What this backend holds for a frame's measurement: the image, and what its values mean.
pub(super) struct MeasuredDepth {
    pub(super) image: DepthImage,
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
/// Every early return in [`EnvironmentDepth::new`] is a case where one thing was made and the next one
/// refused, and a provider the app never gets to see is still a provider the runtime is holding for it -
/// which is not a leak the runtime will clean up at the end, because from its point of view the app made
/// it. So each of them says what it is putting back, and this is the same two calls in the same order as
/// [`Drop for EnvironmentDepth`], with the stop left out because nothing was started yet.
///
/// The `None` is the one return before a swapchain existed at all.
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
        // In the order the spec gives them and not the other way round: the spec says a provider "must not
        // be destroyed until all of its swapchains have been", and this file has exactly one.
        //
        // Stopping first too, because the extension has a stop for a reason - a provider that is still
        // producing frames into a swapchain that is about to go is the one thing here that could race with
        // the runtime rather than merely leak.
        // SAFETY: all three handles are this session's, each is destroyed exactly once, and the order is the
        // spec's - swapchain before provider, because a provider must not be destroyed until its swapchains
        // are. A failure here is nothing to do about: the handles are about to be dropped either way, so
        // what is said is only that it did not go smoothly.
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
