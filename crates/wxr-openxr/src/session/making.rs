//! How a session is made out of a runtime, and every decision taken at that one moment.
//!
//! There is a lot of asking in here and each ask is the same shape: a runtime is told what the app wants
//! and what it will not give is not a reason to fail - it is a session with one fewer thing. So what comes
//! out is the session plus a log line saying what was not had, which is the difference between a headset
//! this backend has not learned and a headset it cannot use.
//!
//! The other half is the extensions, and they are all decided here for one reason: a session's function
//! pointers are its instance's, so whether a session can see a plane is a fact from before the session
//! existed. [`crate::OpenXr`] records which of them the runtime offered and this is where that becomes
//! something the session holds - see [`state::SpatialExtensions`] and its three siblings.

use super::*;

/// A live session, with a stereo swapchain the compositor presents.
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

        // SAFETY: these are a live wgpu device, and the session keeps its own reference to it from here on -
        // which is what the field in `state.rs` is for, and the reason a backend is given a device rather
        // than making one. The check cannot be repeated here: the device is only known to be live because
        // the caller passed it in, and the session holds it so that it stays that way.
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
        let hands = match input::Hands::new(&backend.instance, &session, backend.haptics) {
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

        // The four facilities, each built only where the runtime was made with the extension for it, and
        // each `None` where it was not. `None` is a runtime that has no way to do this, and every one of the
        // four then answers with an honest refusal rather than a handle that never arrives - which is why
        // they are decided here, at the one moment where "this runtime can" is a fact rather than a hope.
        let planes = PlaneDetector::new(&session, backend.spatial.planes);
        let anchors = Anchors::new(&session, backend.spatial.anchors);
        let raycast = Raycaster::new(&session, backend.spatial.raycast);
        let depth = EnvironmentDepth::new(&session, backend.spatial.depth);

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
            kinds: Vec::new(),
            offsets: Vec::new(),
            view,
            located: Vec::new(),
            view_space: None,
            state: wxr::State::Ready,
            visibility: wxr::Visibility::Hidden,
            openxr_state: xr::SessionState::IDLE,
            predicted: xr::Time::from_nanos(0),
            held: None,
            begun: false,
            lost: false,
            foveation: None,
            format,
            layers_enabled: backend.layers,
            refresh_rate: backend.refresh_rate,
            haptics: backend.haptics,
            layers: Vec::new(),
            planes,
            anchors,
            raycast,
            depth,
            measured: None,
            depth_asked: false,
            // Cloned, and the last field - see its documentation for why it is both.
            device: device.clone(),
        })
    }
}
