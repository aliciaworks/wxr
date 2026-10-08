//! The session the runtime handed over, and everything it is asked.

use std::ffi::c_void;
use std::time::Duration;

use openxr as xr;

mod state;

pub(crate) use state::LayerExtensions;

pub use state::OpenXrSession;
use state::*;

use crate::convert::{color_format, field_of_view, pose, posef, reference_space};
use crate::{Device, Error, OpenXr, hal, input};

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
            layers_enabled: backend.layers,
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
        // One bit per extension the instance was made with. A bit set for one that is not enabled would be a
        // promise the session cannot keep - and the refusal in `layer` is what an app gets if it asks anyway.
        for (enabled, bit) in [
            (self.layers_enabled.cylinder, wxr::Features::LAYER_CYLINDER),
            (self.layers_enabled.equirect, wxr::Features::LAYER_EQUIRECT),
            (self.layers_enabled.cube, wxr::Features::LAYER_CUBE),
        ] {
            if enabled {
                features = features.union(bit);
            }
        }
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
        self.poll_impl()
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
        self.space_impl(kind)
    }

    fn offset_space(
        &mut self,
        base: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        self.offset_space_impl(base, offset)
    }

    fn layer(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
    ) -> Result<wxr::Layer, wxr::Error> {
        // A quad is in the core specification; the other three are `XR_KHR_composition_layer_*`, one extension
        // each, and an extension is enabled when the instance is made or not at all. So the refusal is per
        // shape and reads the instance's own answer, which is what the capability bits report.
        let enabled = match shape {
            wxr::LayerShape::Quad { .. } => true,
            wxr::LayerShape::Cylinder { .. } => self.layers_enabled.cylinder,
            wxr::LayerShape::Equirect { .. } => self.layers_enabled.equirect,
            wxr::LayerShape::Cube => self.layers_enabled.cube,
        };
        if !enabled {
            return Err(wxr::Error::Unsupported(format!(
                "{} layers: this runtime does not have the extension",
                shape.name()
            )));
        }
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
                // A cube is six faces of one image and every other shape here is one. The array is one
                // for all of them: a layer is one picture per eye, which is the reason to hand it to a
                // compositor rather than draw it twice.
                face_count: match shape {
                    wxr::LayerShape::Cube => 6,
                    _ => 1,
                },
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
            shape,
            extent: pixels,
            held: None,
        }));
        log::info!(
            "wxr-openxr: a {} layer at {}x{} px",
            shape.name(),
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
        self.begin_impl(_now, out)
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
        self.views_impl(space, out)
    }

    fn inputs(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        self.inputs_impl(space, out)
    }

    fn hand(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        self.hand_impl(source, space, out)
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.end_impl(_frame)
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

mod frames;
mod sources;
mod spaces;
