//! An OpenXR backend for [`wxr`], rendering with wgpu.
//!
//! The whole of this crate is shaped by one decision the core made: **the renderer makes the device, and
//! the session is told about it.** OpenXR offers the other order - `XR_KHR_vulkan_enable2` has the runtime
//! create the `VkInstance` and `VkDevice` for the app to adopt - and that is not the one this is built on,
//! because a runtime that makes the device also gets to decide what the device can do.
//!
//! So a session is created from `XR_KHR_vulkan_enable`, which takes handles that already exist. They come
//! from the wgpu device the renderer was given, read out through wgpu's HAL (`hal`), which is the only
//! place in the workspace that names a Vulkan type. Nothing here creates a device, and nothing here
//! outlives one.
//!
//! The frames are the other half, and they are where the core's [`wxr::Session::Image`] pulls its weight: a
//! session hands out images the compositor will present and nothing else. The renderer wraps them as `wgpu`
//! textures; the core carries them and never looks inside.
//!
//! **Depth is not submitted, and cannot be through this crate.** A runtime takes an app's depth through
//! `XR_KHR_composition_layer_depth`, whose `CompositionLayerDepthInfoKHR` is chained onto each projection view -
//! and `openxr` 0.22 binds neither that structure nor any way to chain one onto a layer: `openxr-sys` has the
//! raw struct, and the crate's builders do not reach it. Doing it means building the layer through the `sys`
//! layer and ending the frame through it, which is a piece of work of its own rather than a line here. What it
//! would buy is what it buys on the other two backends: a compositor that reprojects with depth instead of
//! guessing. It is also why `set_depth_range` is the core's default no-op here rather than an override that
//! stores the planes: there is nothing to submit them with, and a field nothing reads is a field that lies.

#![cfg(not(target_family = "wasm"))]

mod hal;
mod import;
mod input;

pub use import::Images;

use std::ffi::c_void;
use std::time::Duration;

use wxr::glam::{Quat, Vec3};

use openxr as xr;

/// What the renderer hands over: the wgpu objects it made, and owns.
#[derive(Clone, Debug)]
pub struct Device {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
}

/// An OpenXR runtime, connected to but not yet in a session.
pub struct OpenXr {
    instance: xr::Instance,
    system: xr::SystemId,
    views: Vec<xr::ViewConfigurationView>,
    blend: xr::EnvironmentBlendMode,
}

impl OpenXr {
    /// Load the runtime this machine has and take the first headset it offers.
    ///
    /// An `Entry` is the loader rather than a runtime: which runtime it is belongs to the machine, and a
    /// machine with none is [`wxr::Error::Unavailable`] rather than a panic.
    pub fn load() -> Result<Self, Error> {
        // SAFETY: the loader the dynamic loader finds must be an OpenXR one, which is what the runtime
        // the machine has configured is. On platforms with a different loader story this is where that
        // would be said; `()` is the "ask the system" answer everywhere but Android.
        let entry =
            unsafe { xr::Entry::load(&()) }.map_err(|error| Error::runtime("load", error))?;

        // The *legacy* binding, and only it, because it is the one that takes an instance and a device that
        // already exist. Asking for both is not harmless: the crate prefers `XR_KHR_vulkan_enable2` when it
        // is there, and that is the path where the runtime makes the instance itself - the trap this backend
        // is written against. It is also the path where a runtime wants a `vkGetInstanceProcAddr` the app
        // never handed it, and says so: Monado refuses with
        // `xrGetVulkanGraphicsDeviceKHR(sys->vk_get_instance_proc_addr == NULL)`.
        let mut extensions = xr::ExtensionSet::default();
        extensions.khr_vulkan_enable = true;
        // A floor space is worth having and not worth failing over, so it is asked for only from a runtime
        // that lists it: `xrCreateInstance` refuses an extension the runtime does not have, and a session
        // that cannot be created at all is worse than one without a floor.
        extensions.ext_local_floor = entry
            .enumerate_extensions()
            .map_err(|error| Error::runtime("ask what the runtime supports", error))?
            .ext_local_floor;

        // The loader validates this: an application with no name is not an application it will make an
        // instance for, and that is a real check rather than a formality - a runtime's logs are read by
        // whoever has to find out why the headset is not working.
        let app_info = xr::ApplicationInfo {
            application_name: "wxr",
            application_version: 1,
            engine_name: "wxr",
            engine_version: 1,
            ..Default::default()
        };
        let instance = entry
            .create_instance(&app_info, &extensions, &[], &())
            .map_err(|error| Error::runtime("create the instance", error))?;

        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| Error::runtime("find a headset", error))?;

        let views = instance
            .enumerate_view_configuration_views(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|error| Error::runtime("enumerate the views", error))?;
        if views.len() != 2 {
            return Err(Error::Unsupported(format!(
                "the runtime offers {} views; this backend draws two",
                views.len()
            )));
        }

        let blend = *instance
            .enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|error| Error::runtime("enumerate blend modes", error))?
            .first()
            .ok_or_else(|| Error::Unsupported("the runtime offers no blend mode".into()))?;

        Ok(Self {
            instance,
            system,
            views,
            blend,
        })
    }

    /// The Vulkan API version the runtime needs, as `(minimum, maximum)`.
    ///
    /// This has to be asked *before* a device is made: one below the minimum is a session that cannot be
    /// created, and the maximum is what the runtime can be handed. It is the first thing a renderer does,
    /// and the reason `load` and `connect` are two calls rather than one.
    pub fn requirements(&self) -> Result<(xr::Version, xr::Version), Error> {
        let requirements = self
            .instance
            .graphics_requirements::<xr::Vulkan>(self.system)
            .map_err(|error| Error::runtime("ask the graphics requirements", error))?;
        Ok((
            requirements.min_api_version_supported,
            requirements.max_api_version_supported,
        ))
    }

    /// The size the runtime recommends for each eye, which is what a swapchain is made at.
    pub fn recommended_extent(&self) -> wxr::Extent2d {
        wxr::Extent2d::new(
            self.views[0].recommended_image_rect_width,
            self.views[0].recommended_image_rect_height,
        )
    }
}

/// `VK_FORMAT_R8G8B8A8_SRGB`: eight bits each and sRGB-encoded, the format every compositor must accept. A
/// headset wants more than eight bits, and that is a thing to add together with the tone map that makes it
/// usable rather than a format to ask for and hope for.
pub(crate) const RGBA8_SRGB: u32 = 43;

impl wxr::Backend for OpenXr {
    type Device = Device;
    type Session = OpenXrSession;

    fn connect(&self, device: Device) -> Result<OpenXrSession, wxr::Error> {
        OpenXrSession::new(self, &device).map_err(wxr::Error::from)
    }
}

/// What went wrong before a session existed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the OpenXR runtime could not {action}: {message}")]
    Runtime { action: String, message: String },
    #[error("{0}")]
    Unsupported(String),
}

impl Error {
    fn runtime(action: &str, error: impl std::fmt::Debug) -> Self {
        Self::Runtime {
            action: action.to_string(),
            message: format!("{error:?}"),
        }
    }
}

impl From<Error> for wxr::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::Runtime { action, message } => {
                wxr::Error::Unavailable(format!("{action}: {message}"))
            }
            Error::Unsupported(what) => wxr::Error::Rejected(what),
        }
    }
}

/// A live session, with a stereo swapchain the compositor presents.
pub struct OpenXrSession {
    /// The instance is kept for its event queue: OpenXR polls events from the instance, not the session.
    instance: xr::Instance,
    events: xr::EventDataBuffer,
    session: xr::Session<xr::Vulkan>,
    /// Waiting and submitting are two handles, and the wait is where the frame's timing comes from.
    waiter: xr::FrameWaiter,
    stream: xr::FrameStream<xr::Vulkan>,
    swapchain: xr::Swapchain<xr::Vulkan>,
    /// The compositor's images, as the runtime names them - a `VkImage`, which on this platform is an
    /// integer. The renderer wraps these; the core carries them.
    images: Vec<u64>,
    extent: wxr::Extent2d,
    blend: xr::EnvironmentBlendMode,
    spaces: Vec<xr::Space>,
    /// Where each of `spaces` sits inside its kind, because an offset space is the kind's origin moved - and an
    /// offset of an offset has to add up.
    offsets: Vec<wxr::Pose>,
    /// The viewer's own space, whose one job is the head pose: OpenXR reports where the eyes are and not where
    /// the wearer is, and `VIEW` is the single reference space that is the wearer.
    view: xr::Space,
    /// The hands, which are declared once and read once a frame. `None` when the runtime would not take
    /// the action set - a runtime with no controllers is a session with no inputs rather than no session.
    hands: Option<input::Hands>,
    /// The views the runtime located for the frame in progress. Kept because the composition layer is
    /// built from them, and they cannot be recovered from the core's view type without a round trip that
    /// would have to be exact to be honest.
    located: Vec<xr::View>,
    /// The lifecycle and the visibility the core speaks, derived from OpenXR's own ladder below.
    state: wxr::State,
    visibility: wxr::Visibility,
    /// OpenXR's own state, kept because it carries more than the core's two axes do - and a program that
    /// needs the rest needs the platform.
    openxr_state: xr::SessionState,
    predicted: xr::Time,
    /// Which image the frame took, until it is given back.
    held: Option<u32>,
    /// Whether this session has been begun, which is a thing OpenXR makes the app do once.
    begun: bool,
    /// Whether `Lost` has been said, so the end of a session is news once.
    lost: bool,
}

impl OpenXrSession {
    fn new(backend: &OpenXr, device: &Device) -> Result<Self, Error> {
        let native = unsafe { hal::vulkan(&device.instance, &device.device) }.ok_or_else(|| {
            Error::Unsupported(
                "the device is not a Vulkan one, and OpenXR's binding on this platform is".into(),
            )
        })?;

        // The runtime is asked which physical device it wants *before* a session is made with it, and that
        // is not a formality: one that has not been asked refuses the session outright, which is what
        // Monado says - `Has not called xrGetVulkanGraphicsDeviceKHR`. The answer is the GPU the renderer
        // already made its device on, and when it is not, saying so is better than a session created
        // against the wrong one - the whole reason the renderer makes the device in the first place.
        // SAFETY: the instance is live, and the handle is the one this device was made from.
        let wanted = unsafe {
            backend
                .instance
                .vulkan_graphics_device(backend.system, native.instance as *mut c_void)
        }
        .map_err(|error| Error::runtime("ask which Vulkan device the runtime wants", error))?;
        if !std::ptr::eq(wanted, native.physical_device) {
            return Err(Error::Unsupported(format!(
                "the runtime wants the Vulkan device {wanted:?}, and the renderer's was made on {:?}",
                native.physical_device
            )));
        }

        let info = xr::vulkan::SessionCreateInfo {
            instance: native.instance,
            physical_device: native.physical_device,
            device: native.device,
            queue_family_index: native.queue_family_index,
            queue_index: native.queue_index,
        };

        // SAFETY: these are a live wgpu device, and the caller keeps it alive for as long as the session -
        // which is the contract `hal::vulkan` documents, and the reason a backend is given a device rather
        // than making one.
        let (session, waiter, stream) = unsafe {
            backend
                .instance
                .create_session::<xr::Vulkan>(backend.system, &info)
        }
        .map_err(|error| Error::runtime("create a session", error))?;

        let formats = session
            .enumerate_swapchain_formats()
            .map_err(|error| Error::runtime("enumerate swapchain formats", error))?;
        let format = formats
            .iter()
            .find(|format| **format == RGBA8_SRGB)
            .or_else(|| formats.first())
            .copied()
            .ok_or_else(|| Error::Unsupported("the runtime offers no swapchain format".into()))?;

        let extent = backend.recommended_extent();
        let swapchain = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format,
                sample_count: 1,
                width: extent.width,
                height: extent.height,
                face_count: 1,
                // Two eyes, one array layer each: the compositor presents one image and reads a layer per
                // eye, which is what makes `ImageMeta::layers` two.
                array_size: 2,
                mip_count: 1,
            })
            .map_err(|error| Error::runtime("create the swapchain", error))?;

        let images = swapchain
            .enumerate_images()
            .map_err(|error| Error::runtime("enumerate the swapchain's images", error))?;

        // Inputs are declared here and not in the constructor: they need a session, and a runtime that will
        // not take them is a session without hands rather than a session that failed.
        let hands = match input::Hands::new(&backend.instance, &session) {
            Ok(hands) => Some(hands),
            Err(error) => {
                log::warn!("wxr-openxr: no inputs: {error}");
                None
            }
        };

        // Created once and located every frame, because `VIEW` is the head and the head is what a scene that
        // wants the camera on the viewer asks for.
        let view = session
            .create_reference_space(xr::ReferenceSpaceType::VIEW, xr::Posef::IDENTITY)
            .map_err(|error| Error::runtime("create the viewer space", error))?;

        Ok(Self {
            instance: backend.instance.clone(),
            events: xr::EventDataBuffer::new(),
            session,
            waiter,
            stream,
            swapchain,
            images,
            hands,
            extent,
            blend: backend.blend,
            spaces: Vec::new(),
            offsets: Vec::new(),
            view,
            located: Vec::new(),
            state: wxr::State::Ready,
            visibility: wxr::Visibility::Hidden,
            openxr_state: xr::SessionState::IDLE,
            predicted: xr::Time::from_nanos(0),
            held: None,
            begun: false,
            lost: false,
        })
    }
}

impl wxr::Session for OpenXrSession {
    type Image = u64;

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.state
    }

    fn visibility(&self) -> wxr::Visibility {
        self.visibility
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        // The press edges the last frame produced come first: they are this crate's own, and waiting for the
        // runtime to get round to an instance event to report them would be a delay with no cause.
        if let Some(hands) = self.hands.as_mut()
            && let Some(event) = hands.poll()
        {
            return Some(event);
        }

        // Events come off the instance, one at a time. A state change is the one a frame loop acts on and a
        // profile change is the set of inputs changing under it; the rest are the runtime's business, and
        // skipping them is better than inventing a mapping for them.
        loop {
            match self.instance.poll_event(&mut self.events) {
                // Which controllers the runtime has bound, which is the set of inputs changing: what they then
                // *are* is the next frame's answer, from `inputs`.
                Ok(Some(xr::Event::InteractionProfileChanged(_))) => {
                    return Some(wxr::Event::InputsChanged);
                }
                Ok(Some(xr::Event::SessionStateChanged(event))) => {
                    let openxr = event.state();
                    self.openxr_state = openxr;
                    // OpenXR's ladder is where both of the core's axes are read from, and the interesting
                    // rung is `VISIBLE` without `FOCUSED`: a session on a display that nobody is attending
                    // to, which is exactly WebXR's `visible-blurred`.
                    // `STOPPING` is the runtime asking the app to end the session rather than the session
                    // already being over: the one thing `ExitRequested` is for, and the app is expected to
                    // shut down cleanly rather than have it done for it. The session is still here, so it is
                    // still `Ready` - just not on a display any more.
                    if openxr == xr::SessionState::STOPPING {
                        self.state = wxr::State::Ready;
                        self.visibility = wxr::Visibility::Hidden;
                        return Some(wxr::Event::ExitRequested);
                    }
                    let (state, visibility) = match openxr {
                        xr::SessionState::READY => {
                            // A session runs only after the app has begun it, and only once it is ready -
                            // the runtime says `XR_ERROR_SESSION_NOT_RUNNING` from `xrWaitFrame` until then,
                            // which is the one thing it will not do on the app's behalf.
                            if !self.begun {
                                if let Err(error) = self
                                    .session
                                    .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                {
                                    log::error!("wxr-openxr: beginning the session: {error:?}");
                                }
                                self.begun = true;
                            }
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::IDLE | xr::SessionState::SYNCHRONIZED => {
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::VISIBLE => {
                            (wxr::State::Ready, wxr::Visibility::VisibleBlurred)
                        }
                        xr::SessionState::FOCUSED => (wxr::State::Ready, wxr::Visibility::Visible),
                        xr::SessionState::LOSS_PENDING | xr::SessionState::EXITING => {
                            (wxr::State::Ended, wxr::Visibility::Hidden)
                        }
                        // A state this crate has not learned is not a state to guess at: the session is
                        // still whatever it was, and the next event will say what happened.
                        _ => continue,
                    };
                    if state != self.state {
                        self.state = state;
                        return Some(wxr::Event::StateChanged(state));
                    }
                    if visibility != self.visibility {
                        self.visibility = visibility;
                        return Some(wxr::Event::VisibilityChanged(visibility));
                    }
                    continue;
                }
                Ok(Some(_)) => continue,
                Ok(None) => {
                    // A session the runtime has taken away is `Lost` once, after the state that said so: a frame
                    // loop that keeps polling is told the difference between over and gone.
                    if self.state == wxr::State::Ended && !self.lost {
                        self.lost = true;
                        return Some(wxr::Event::Lost);
                    }
                    return None;
                }
                Err(error) => {
                    log::error!("wxr-openxr: polling events: {error:?}");
                    self.state = wxr::State::Ended;
                    self.lost = true;
                    return Some(wxr::Event::Lost);
                }
            }
        }
    }

    /// What the display shows behind the picture, which the runtime said when the session was made.
    fn blend(&self) -> wxr::Blend {
        match self.blend {
            xr::EnvironmentBlendMode::OPAQUE => wxr::Blend::Opaque,
            xr::EnvironmentBlendMode::ADDITIVE => wxr::Blend::Additive,
            xr::EnvironmentBlendMode::ALPHA_BLEND => wxr::Blend::AlphaBlend,
            _ => wxr::Blend::Opaque,
        }
    }

    fn images(&self) -> wxr::ImageMeta {
        wxr::ImageMeta {
            format: wxr::ColorFormat::Rgba8Srgb,
            extent: self.extent,
            layers: 2,
        }
    }

    fn image_count(&self) -> usize {
        1
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        // The image this frame acquired, which is the one the compositor will present - not the first of the
        // swapchain's, which is a different image on most frames and is not this frame's at all.
        (index == 0)
            .then(|| self.images.get(self.held? as usize))
            .flatten()
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let space = self
            .session
            .create_reference_space(reference_space(kind), xr::Posef::IDENTITY)
            .map_err(|_| wxr::Error::NoSpace(kind))?;
        self.spaces.push(space);
        self.offsets.push(wxr::Pose::IDENTITY);
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn offset_space(
        &mut self,
        base: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        if self.spaces.get(base.id() as usize).is_none() {
            return Err(wxr::Error::NoSpace(base.kind));
        }
        // A reference space here is a type and a pose inside it, so an offset of one is that pose composed with
        // this one - and the result is a space of the same type, which is what keeps it tracking the room.
        let inside = self.offsets[base.id() as usize].then(offset);
        let space = self
            .session
            .create_reference_space(reference_space(base.kind), posef(inside))
            .map_err(|_| wxr::Error::NoSpace(base.kind))?;
        self.spaces.push(space);
        self.offsets.push(inside);
        Ok(wxr::ReferenceSpace::new(
            base.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // The runtime is asked to wait, and it answers with the frame's timing: when the picture will be
        // shown, and whether there is anything to draw at all. `now` is not used - OpenXR's clock is the
        // runtime's, and comparing a wall clock against it is a comparison between two unrelated epochs.
        let state = self
            .waiter
            .wait()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        self.predicted = state.predicted_display_time;

        out.state = if state.should_render {
            wxr::FrameState::Render
        } else {
            wxr::FrameState::Wait
        };
        out.predicted_display_time =
            Duration::from_nanos(state.predicted_display_time.as_nanos().max(0) as u64);
        out.views_mut().clear();

        if out.state != wxr::FrameState::Render {
            return Ok(());
        }

        // Once a frame and before anything is read: OpenXR resolves the bindings here, and a pose read
        // before this is the pose from the frame before.
        if let Some(hands) = &self.hands
            && let Err(error) = hands.sync(&self.session)
        {
            log::debug!("wxr-openxr: syncing the actions: {error}");
        }

        self.stream
            .begin()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;

        let held = self
            .swapchain
            .acquire_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        // Unbounded: the runtime is the one that knows when the image is free, and a timeout here would be
        // this code deciding it knows better.
        self.swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        self.held = Some(held);
        Ok(())
    }

    fn views(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let (_, located) = self
            .session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                self.predicted,
                reference,
            )
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        if located.len() != 2 {
            return Err(wxr::Error::Present(
                "the runtime located an odd number of views".into(),
            ));
        }

        // The head, in the same space the views are in - the eyes are placed around it, and a scene that wants
        // the camera on the wearer rather than on an eye asks for it.
        if let Ok(location) = self.view.locate(reference, self.predicted) {
            out.viewer = pose(location.pose);
        }

        let image = self.held.unwrap_or(0);
        let views = out.views_mut();
        for (index, view) in located.iter().enumerate() {
            views.push(wxr::View {
                eye: if index == 0 {
                    wxr::Eye::Left
                } else {
                    wxr::Eye::Right
                },
                pose: pose(view.pose),
                fov: field_of_view(view.fov),
                viewport: wxr::Viewport {
                    x: 0,
                    y: 0,
                    width: self.extent.width,
                    height: self.extent.height,
                },
                image: image as usize,
                layer: index as u32,
            });
        }
        self.located = located;
        Ok(())
    }

    fn inputs(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        let Some(hands) = self.hands.as_mut() else {
            return Ok(());
        };
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        hands.read(&self.session, reference, self.predicted, out);
        Ok(())
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.held = None;
        self.swapchain
            .release_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;

        // The layer borrows this frame's views, so it is built here and lives only until it is submitted.
        // Each eye is a rectangle of the one image, in its own array layer.
        let extent = xr::Extent2Di {
            width: self.extent.width as i32,
            height: self.extent.height as i32,
        };
        let views: Vec<_> = self
            .located
            .iter()
            .enumerate()
            .map(|(index, view)| {
                xr::CompositionLayerProjectionView::new()
                    .pose(view.pose)
                    .fov(view.fov)
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchain)
                            .image_array_index(index as u32)
                            .image_rect(xr::Rect2Di {
                                offset: xr::Offset2Di { x: 0, y: 0 },
                                extent,
                            }),
                    )
            })
            .collect();
        let layer = xr::CompositionLayerProjection::new()
            .space(&self.spaces[0])
            .views(&views);

        self.stream
            .end(self.predicted, self.blend, &[&layer])
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        Ok(())
    }
}

impl OpenXrSession {
    /// OpenXR's own session state, which says more than the core's two axes do.
    ///
    /// It is the same thing [`wxr::Session::as_backend`] reaches, kept as a method because reaching it is
    /// common enough here to be worth not spelling out: a program that already has an `OpenXrSession` should
    /// not have to go through `Any` to ask it what OpenXR says.
    pub fn openxr_state(&self) -> xr::SessionState {
        self.openxr_state
    }
}

/// Which OpenXR reference space a core one is.
fn reference_space(kind: wxr::SpaceKind) -> xr::ReferenceSpaceType {
    match kind {
        wxr::SpaceKind::Viewer => xr::ReferenceSpaceType::VIEW,
        wxr::SpaceKind::Local => xr::ReferenceSpaceType::LOCAL,
        wxr::SpaceKind::LocalFloor => xr::ReferenceSpaceType::LOCAL_FLOOR,
        // `STAGE` is the floor *and* the room the user walked in to define it, which is what bounded means.
        wxr::SpaceKind::BoundedFloor => xr::ReferenceSpaceType::STAGE,
        // An unbounded space is an extension; a runtime without it has `LOCAL`, and a caller that asked for
        // unbounded has got the space it asked for as closely as it exists.
        wxr::SpaceKind::Unbounded => xr::ReferenceSpaceType::LOCAL,
    }
}

/// The core's pose as OpenXR's, which is the direction an offset space is made in.
fn posef(pose: wxr::Pose) -> xr::Posef {
    xr::Posef {
        orientation: xr::Quaternionf {
            x: pose.orientation.x,
            y: pose.orientation.y,
            z: pose.orientation.z,
            w: pose.orientation.w,
        },
        position: xr::Vector3f {
            x: pose.position.x,
            y: pose.position.y,
            z: pose.position.z,
        },
    }
}

/// An OpenXR pose in the core's terms.
fn pose(pose: xr::Posef) -> wxr::Pose {
    wxr::Pose {
        position: Vec3::new(pose.position.x, pose.position.y, pose.position.z),
        orientation: Quat::from_xyzw(
            pose.orientation.x,
            pose.orientation.y,
            pose.orientation.z,
            pose.orientation.w,
        ),
    }
}

/// An OpenXR field of view in the core's terms.
///
/// OpenXR gives the four directions as angles from the centre, with up and right positive; the core keeps
/// them as the four openings, which is what a projection is built from.
fn field_of_view(fov: xr::Fovf) -> wxr::FieldOfView {
    wxr::FieldOfView {
        up: fov.angle_up,
        down: -fov.angle_down,
        left: fov.angle_left,
        right: -fov.angle_right,
    }
}
