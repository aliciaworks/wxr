//! The session the runtime handed over, and everything it is asked.

use std::ffi::c_void;
use std::time::Duration;

use openxr as xr;

use crate::convert::{color_format, field_of_view, pose, posef, reference_space};
use crate::{Device, Error, OpenXr, hal, input};

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
    /// What the swapchain's images are. This backend chooses it from what the runtime offers, so it is the one
    /// that has to report it - and the renderer builds its pipeline from the answer.
    color: wxr::ColorFormat,
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
    /// The foveation profile the app last asked for, kept because the swapchain points at it - dropping it
    /// would leave the swapchain with a dangling one.
    foveation: Option<xr::FoveationProfileFB>,
    /// The swapchain format as the runtime names it, kept because a layer's swapchain has to be made in a
    /// format the runtime offered too - and the same one is the honest choice: the renderer's pipeline is
    /// built once.
    format: u32,
    /// The layers the app made, each `None` once released. The index is the handle's id, so a release leaves a
    /// hole rather than shifting the ones after it.
    layers: Vec<Option<Layer>>,
}

/// A layer the app made, and the swapchain it draws into.
///
/// One swapchain per layer, which is what a composition layer *is* on OpenXR: the runtime reads the image the
/// layer names at `xrEndFrame`, and a swapchain is how an app hands one over without the compositor and the
/// renderer sharing a fence. The projection layer is the same machinery and is not one of these, because a
/// session has it whether an app asked for one or not.
struct Layer {
    swapchain: xr::Swapchain<xr::Vulkan>,
    /// Its images, as the runtime names them - a `VkImage`, which on this platform is an integer - for the
    /// renderer to wrap, the same shape the eyes' images arrive in.
    images: Vec<u64>,
    /// The space it was made in, by the session's own index, and where in it. Both are needed at submission,
    /// because a layer's pose is read then and not when it is placed.
    space: usize,
    pose: xr::Posef,
    /// The shape in the terms `xrEndFrame` wants: the size in metres for a quad.
    size: xr::Extent2Df,
    /// And the image's size in pixels, which is the resolution the app asked to draw at - a different fact, and
    /// the one a renderer makes its target from.
    extent: wxr::Extent2d,
    /// Which image this frame took, until it is given back.
    held: Option<u32>,
}

impl OpenXrSession {
    pub(crate) fn new(backend: &OpenXr, device: &Device) -> Result<Self, Error> {
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

        // The runtime will not make a session until it has been asked what Vulkan version it needs: Monado
        // refuses with `Has not called xrGetVulkanGraphicsRequirementsKHR`. That query is how an app *decides*
        // which device to make, so it is an app's to call first - and it is asked again here, because a session
        // that cannot be created over a precondition the backend could have met is a footgun rather than a
        // contract. Asking twice is a query, not a side effect.
        backend.requirements()?;

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
        // An HDR format first when the app asked for one, and sRGB 8-bit otherwise - which is what every
        // compositor must accept. Failing that, the first format this core can *name*, because a format it
        // cannot name is a frame it will not draw; and only then whatever is left, reported as `Unknown` rather
        // than as a plausible lie about what the pixels mean.
        let format = formats
            .iter()
            .copied()
            .find(|format| backend.prefer_hdr && color_format(*format).is_hdr())
            .or_else(|| {
                formats
                    .iter()
                    .copied()
                    .find(|format| *format == ash::vk::Format::R8G8B8A8_SRGB.as_raw() as u32)
            })
            .or_else(|| {
                formats
                    .iter()
                    .copied()
                    .find(|format| color_format(*format) != wxr::ColorFormat::Unknown)
            })
            .or_else(|| formats.first().copied())
            .ok_or_else(|| Error::Unsupported("the runtime offers no swapchain format".into()))?;
        let color = color_format(format);

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
            color,
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
            foveation: None,
            format,
            layers: Vec::new(),
        })
    }
}

impl wxr::Session for OpenXrSession {
    type Image = u64;
    /// No depth to hand over: the binding cannot chain a depth layer, so there is no measurement to carry.
    type Depth = ();

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.state
    }

    fn visibility(&self) -> wxr::Visibility {
        self.visibility
    }

    fn features(&self) -> wxr::Features {
        // A quad layer is in the core specification rather than behind an extension, and a swapchain is the
        // only thing one needs - so every session of this backend has a quad, which is the one shape it makes.
        // The other three are `XR_KHR_composition_layer_*`, and the bits for them stay unset until those are
        // enabled and driven.
        let mut features = wxr::Features::LAYER_QUAD;
        // A hand tracker is the skeleton, so a session that was given one has hand tracking. Everything else
        // this backend asks for - the depth layer, surfaces - it does not get.
        if let Some(hands) = &self.hands
            && hands.has_tracking()
        {
            features = features.union(wxr::Features::HAND_TRACKING);
        }
        features
    }

    fn set_foveation(&mut self, amount: f32) {
        // `XR_FB_foveation` has four levels rather than a fraction, so an amount becomes the nearest of them.
        // A runtime that does not list the extension is a runtime that does not foveate, and a profile is
        // where that is found out - which is a knob left alone rather than a frame to fail.
        let level = match amount.clamp(0.0, 1.0) {
            a if a <= 0.0 => xr::FoveationLevelFB::NONE,
            a if a < 0.34 => xr::FoveationLevelFB::LOW,
            a if a < 0.67 => xr::FoveationLevelFB::MEDIUM,
            _ => xr::FoveationLevelFB::HIGH,
        };
        let profile = match self
            .session
            .create_foveation_profile(Some(xr::FoveationLevelProfile {
                level,
                vertical_offset: 0.0,
                dynamic: xr::FoveationDynamicFB::DISABLED,
            })) {
            Ok(profile) => profile,
            Err(error) => {
                log::debug!("wxr-openxr: no foveation for {amount}: {error:?}");
                return;
            }
        };

        // The typed crate makes a profile and has no way to put one on a swapchain, so this is the one raw call
        // in this backend: `xrUpdateSwapchainFB` from `XR_FB_swapchain_update_state`, which is what a profile is
        // for.
        let Some(update) = self.instance.exts().fb_swapchain_update_state.as_ref() else {
            return;
        };
        // The typed crate makes a profile and keeps the swapchain state that carries it private, so this is the
        // one raw structure in this backend. Zeroed is the whole of what it starts as: empty flags, no chain.
        let mut state: openxr_sys::SwapchainStateFoveationFB = unsafe { std::mem::zeroed() };
        state.ty = openxr_sys::StructureType::SWAPCHAIN_STATE_FOVEATION_FB;
        state.profile = profile.as_raw();
        // SAFETY: the swapchain is live for as long as this session is, and `state` and `profile` outlive the
        // call - the profile by being kept below, which is what `foveation` is for.
        let result = unsafe {
            (update.update_swapchain)(self.swapchain.as_raw(), &state as *const _ as *const _)
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: the swapchain would not take foveation {amount}: {result:?}");
            return;
        }
        self.foveation = Some(profile);
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
            format: self.color,
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

    fn layer(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
    ) -> Result<wxr::Layer, wxr::Error> {
        // A quad is in the core specification; the other three are `XR_KHR_composition_layer_*`, which this
        // instance does not enable - so the refusal is per shape, which is how the platforms state it and what
        // the capability bits are for.
        let size = match shape {
            wxr::LayerShape::Quad { width, height } => xr::Extent2Df { width, height },
            other => return Err(wxr::Error::Unsupported(format!("{} layers", other.name()))),
        };
        if self.spaces.get(space.id() as usize).is_none() {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        let swapchain = self
            .session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                // The format the session is drawing the world in, which is the one the runtime offered and the
                // one the renderer's pipeline is already built for.
                format: self.format,
                sample_count: 1,
                width: pixels.width,
                height: pixels.height,
                face_count: 1,
                // One: a quad is one picture for both eyes, which is the whole reason to hand it to a
                // compositor instead of drawing it twice.
                array_size: 1,
                mip_count: 1,
            })
            .map_err(|error| Error::runtime("create a layer's swapchain", error))?;
        let images = swapchain
            .enumerate_images()
            .map_err(|error| Error::runtime("enumerate a layer's images", error))?;
        let id = self.layers.len() as u32;
        self.layers.push(Some(Layer {
            swapchain,
            images,
            space: space.id() as usize,
            pose: xr::Posef::IDENTITY,
            size,
            extent: pixels,
            held: None,
        }));
        log::info!(
            "wxr-openxr: a quad layer, {}x{} m at {}x{} px",
            size.width,
            size.height,
            pixels.width,
            pixels.height
        );
        Ok(wxr::Layer::new(id))
    }

    fn layer_image(&mut self, layer: wxr::Layer) -> Option<(&Self::Image, wxr::LayerImage)> {
        let slot = self.layers.get(layer.id() as usize)?.as_ref()?;
        let held = slot.held?;
        let image = slot.images.get(held as usize)?;
        // One image and the whole of it: a layer's swapchain is the layer's, so there is no atlas to take a
        // sub-rectangle out of - which is the difference between this and the projection layer's sub-image.
        Some((
            image,
            wxr::LayerImage {
                meta: wxr::ImageMeta {
                    format: self.color,
                    extent: slot.extent,
                    layers: 1,
                },
                viewport: wxr::Viewport {
                    x: 0,
                    y: 0,
                    width: slot.extent.width,
                    height: slot.extent.height,
                },
            },
        ))
    }

    fn set_layer_pose(&mut self, layer: wxr::Layer, pose: wxr::Pose) -> Result<(), wxr::Error> {
        let Some(slot) = self
            .layers
            .get_mut(layer.id() as usize)
            .and_then(Option::as_mut)
        else {
            return Err(wxr::Error::Unsupported("no such layer".into()));
        };
        // Kept rather than applied: a composition layer's pose is read at `xrEndFrame`, so placing one is not a
        // call into the runtime - it is where the next frame will say it is.
        slot.pose = posef(pose);
        Ok(())
    }

    fn release_layer(&mut self, layer: wxr::Layer) {
        // Dropping the swapchain is the whole of it: OpenXR has no `destroy` to call, and a layer the app stops
        // naming at `xrEndFrame` has stopped being presented.
        if let Some(slot) = self.layers.get_mut(layer.id() as usize) {
            *slot = None;
        }
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // A runtime is not asked to wait for a frame until the session has been begun. `xrWaitFrame` before
        // `xrBeginSession` is an error at best, and the event that says to begin arrives on the same poll the
        // caller is not making while it waits - so waiting on it is a deadlock rather than a frame. A session
        // that has not begun is a frame to wait for.
        if !self.begun {
            out.views_mut().clear();
            out.state = wxr::FrameState::Wait;
            return Ok(());
        }

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

        // Each layer has a swapchain of its own, and each is taken in the same frame the eyes are: an image the
        // runtime has not handed over is not one an app may draw into, and waiting is unbounded for the reason
        // the eyes' wait is - the runtime is the one that knows when it is free.
        for layer in self.layers.iter_mut().flatten() {
            let held = layer
                .swapchain
                .acquire_image()
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            layer
                .swapchain
                .wait_image(xr::Duration::INFINITE)
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            layer.held = Some(held);
        }
        Ok(())
    }

    fn binding(&mut self, layer: wxr::Layer) -> Result<wxr::Binding, wxr::Error> {
        // On OpenXR a composition layer *is* its swapchain - the runtime reads the image the layer names at
        // `xrEndFrame` - so there is nothing to make here: the binding is the layer's own, named by the handle
        // the layer already has.
        Ok(wxr::Binding::new(layer.id()))
    }

    fn sub_image(&mut self, binding: wxr::Binding, view: usize) -> Option<wxr::SubImage> {
        // The swapchain the layer draws into: the session's own for the projection layer, the layer's own
        // otherwise. Either way the extent is the whole image.
        let extent = self
            .layers
            .get(binding.id() as usize)
            .and_then(|slot| slot.as_ref())
            .map(|layer| layer.extent)
            .unwrap_or(self.extent);
        Some(wxr::SubImage {
            color_size: wxr::glam::UVec2::new(extent.width, extent.height),
            // The session's swapchain is a colour attachment and sampled with no depth image of its own, so
            // there is no depth size to report rather than a zero one.
            depth_size: None,
            // A stereo swapchain carries the eyes as its array slices, not as two viewports of one image -
            // which is why `ImageMeta::layers` is two - so each eye owns the whole of its slice.
            viewport: wxr::Viewport {
                x: 0,
                y: 0,
                width: extent.width,
                height: extent.height,
            },
            array_index: Some(view as u32),
        })
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
                recommended_viewport_scale: None,
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

    fn hand(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        let Some(hands) = self.hands.as_ref() else {
            out.clear();
            return Ok(());
        };
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        hands.hand(source.get() as usize, reference, self.predicted, out);
        Ok(())
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.held = None;
        self.swapchain
            .release_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        for layer in self.layers.iter_mut().flatten() {
            if layer.held.take().is_some() {
                layer
                    .swapchain
                    .release_image()
                    .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            }
        }

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

        // The app's layers, in the order they were made: a quad is a rectangle at a pose, and the whole of its
        // swapchain image is the picture. They are built here and not kept, because a sub-image borrows the
        // swapchain and this is the one place a frame is submitted.
        let quads: Vec<xr::CompositionLayerQuad<'_, xr::Vulkan>> = self
            .layers
            .iter()
            .flatten()
            .filter_map(|layer| {
                Some(
                    xr::CompositionLayerQuad::new()
                        // A layer is a picture *over* what is already there, so its alpha is part of the
                        // picture: without this bit the compositor ignores the channel and a panel with a
                        // transparent background is a black rectangle. Opaque content says the same thing with
                        // every alpha at one.
                        .layer_flags(xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA)
                        .space(self.spaces.get(layer.space)?)
                        .eye_visibility(xr::EyeVisibility::BOTH)
                        .pose(layer.pose)
                        .size(layer.size)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&layer.swapchain)
                                .image_array_index(0)
                                .image_rect(xr::Rect2Di {
                                    offset: xr::Offset2Di { x: 0, y: 0 },
                                    extent: xr::Extent2Di {
                                        width: layer.extent.width as i32,
                                        height: layer.extent.height as i32,
                                    },
                                }),
                        ),
                )
            })
            .collect();

        // The world first and the app's pictures over it: the projection layer is what a scene is drawn into,
        // and a panel that arrived behind it would be a panel nobody sees.
        let mut present: Vec<&xr::CompositionLayerBase<'_, xr::Vulkan>> =
            Vec::with_capacity(1 + quads.len());
        present.push(&layer);
        for quad in &quads {
            present.push(quad);
        }

        self.stream
            .end(self.predicted, self.blend, &present)
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
