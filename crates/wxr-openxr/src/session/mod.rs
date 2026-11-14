//! The session the runtime handed over, and everything it is asked.

use std::ffi::c_void;
use std::time::Duration;

use openxr as xr;

mod anchors;
mod depth;
mod hits;
mod layers;
mod planes;
mod state;

// The types the four world-modelling modules make, named here so `state` can hold them and the trait block
// below can point at them. `DepthImage` is public because it is this backend's `Session::Depth` - a caller
// has to be able to name what a depth query hands back, however it reads it.
use anchors::Anchors;
use depth::{EnvironmentDepth, MeasuredDepth};
use hits::Raycaster;
use planes::PlaneDetector;

pub(crate) use state::{HapticExtensions, InputExtensions, LayerExtensions, SpatialExtensions};

pub use depth::DepthImage;
pub use state::OpenXrSession;
use state::*;

use crate::convert::{color_format, field_of_view, pose, posef, reference_space};
use crate::{Device, Error, OpenXr, hal, input};

/// Whether a runtime stands behind the place it reported.
///
/// Both halves of the pose have to be valid, and the `_TRACKED` bits are deliberately not asked for: they say
/// whether tracking is *active*, and a valid pose that is momentarily untracked is still a place the runtime
/// stands behind - the difference between a frame that lost sight of the wall and one that cannot say at all.
fn is_located(flags: openxr_sys::SpaceLocationFlags) -> bool {
    use openxr_sys::SpaceLocationFlags as Bits;
    flags.contains(Bits::POSITION_VALID) && flags.contains(Bits::ORIENTATION_VALID)
}

mod making;

impl wxr::Session for OpenXrSession {
    type Image = u64;
    /// The runtime's own depth map for this frame, which is a texture the *scene* samples rather than a
    /// depth layer to submit - `XR_META_environment_depth` measures the room for the app to draw against.
    type Depth = DepthImage;

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
        self.features_impl()
    }

    /// The rates the runtime offers, which is `xrEnumerateDisplayRefreshRatesFB`.
    ///
    /// A runtime without the extension offers none, and a caller that finds the list empty keeps the rate the
    /// display is already at.
    fn refresh_rates(&mut self, out: &mut Vec<f32>) {
        self.refresh_rates_impl(out)
    }

    /// The rate the display is running at, which is `xrGetDisplayRefreshRateFB`.
    ///
    /// A runtime without the extension has one rate and no way to ask, so `None` is the whole answer.
    fn refresh_rate(&mut self) -> Option<f32> {
        self.refresh_rate_impl()
    }

    /// Ask for one of them, which is `xrRequestDisplayRefreshRateFB`.
    ///
    /// A runtime that will not take this one keeps the rate it has, so a refusal is logged rather than
    /// returned - the same shape `set_foveation` has, and for the same reason: it is a request.
    fn set_refresh_rate(&mut self, rate: f32) {
        self.set_refresh_rate_impl(rate)
    }

    fn set_foveation(&mut self, amount: f32) {
        self.set_foveation_impl(amount)
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

    fn bounds(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        self.bounds_impl(space, out)
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
            return Err(wxr::Error::Unsupported(
                format!(
                    "{} layers: this runtime does not have the extension",
                    shape.name()
                )
                .into(),
            ));
        }
        // The kind this session recorded for the id, not the public half of the handle: a layer keeps the
        // kind because a translated cube is placed by making a reference space of that kind. Both halves are
        // checked first, though, because a kind the caller changed is not a space this session handed out -
        // answering `NoSpace` is the same thing every other entry taking a space says about it.
        let Some(space_id) = self.space_index(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let space_kind = self.kinds[space_id];
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
            space: space_id,
            space_kind,
            pose: xr::Posef::IDENTITY,
            placed: None,
            placed_at: None,
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
                    // A cube's swapchain is six faces, one per array layer. Every other shape is one picture.
                    layers: if matches!(slot.shape, wxr::LayerShape::Cube) {
                        6
                    } else {
                        1
                    },
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
        // call into the runtime - it is where the next frame will say it is. A cube is the exception: its
        // composition struct has an orientation and no position, so a translated cube is placed by making a
        // reference space at that translation and handing the cube that space instead.
        if matches!(slot.shape, wxr::LayerShape::Cube) && pose.position != wxr::glam::Vec3::ZERO {
            if slot.placed.is_none() || slot.placed_at != Some(pose.position) {
                let Some(offset) = self.offsets.get(slot.space).copied() else {
                    return Err(wxr::Error::NoSpace(slot.space_kind));
                };
                let at = wxr::Pose {
                    position: pose.position,
                    orientation: wxr::glam::Quat::IDENTITY,
                };
                let space = self
                    .session
                    .create_reference_space(
                        reference_space(slot.space_kind),
                        posef(offset.then(at)),
                    )
                    .map_err(|error| Error::runtime("place a cube layer", error))?;
                slot.placed = Some(space);
                slot.placed_at = Some(pose.position);
            }
        } else {
            slot.placed = None;
            slot.placed_at = None;
        }
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
        self.binding_impl(layer)
    }

    fn sub_image(&mut self, binding: wxr::Binding, view: usize) -> Option<wxr::SubImage> {
        self.sub_image_impl(binding, view)
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

    /// The interaction profile the runtime reports for one source, which is
    /// `xrGetCurrentInteractionProfile` - and the eyes' fixed one.
    fn profiles(&mut self, source: wxr::InputId, out: &mut Vec<String>) -> Result<(), wxr::Error> {
        self.profiles_impl(source, out)
    }

    /// A vibration on one source, which is `xrApplyHapticFeedback` with a `XrHapticVibration`. A source with
    /// no output path - and every source on a runtime without the extension - is `Unsupported` rather than a
    /// silent nothing: this is a call the app chose to make, so an answer it can act on is worth more.
    fn pulse(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        let Some(hands) = &self.hands else {
            return Err(wxr::Error::Unsupported("haptics".into()));
        };
        hands
            .pulse(&self.session, source, intensity, duration)
            .map_err(wxr::Error::from)
    }

    /// A vibration taken back, which is `xrStopHapticFeedback` on the same action.
    fn stop_pulse(&mut self, source: wxr::InputId) -> Result<(), wxr::Error> {
        let Some(hands) = &self.hands else {
            return Err(wxr::Error::Unsupported("haptics".into()));
        };
        hands.stop(&self.session, source).map_err(wxr::Error::from)
    }

    /// A waveform on one source, which is `XR_FB_haptic_pcm`'s vibration.
    fn play_pcm(
        &mut self,
        source: wxr::InputId,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<(), wxr::Error> {
        let Some(hands) = &self.hands else {
            return Err(wxr::Error::Unsupported("haptic samples".into()));
        };
        hands
            .play_pcm(&self.session, source, samples, sample_rate)
            .map_err(wxr::Error::from)
    }

    /// The surfaces the runtime has found this frame, which is `XR_EXT_plane_detection`'s state machine.
    ///
    /// Looking for planes is asynchronous on OpenXR: a pass is begun, polled until it is done, read, and begun
    /// again - so one call here is one *step* of that loop, and an empty list is a pass that has not finished.
    /// That is the shape the frame's views have, and for the same reason: this frame's, not a list to keep.
    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        self.planes_impl(space, out)
    }

    /// Ask the runtime to keep a place fixed in the room, which is `XR_FB_spatial_entity`'s anchor.
    ///
    /// Made asynchronously by OpenXR too, so what comes back names a slot that fills in when the runtime
    /// answers - and `anchor_pose` on one that has not is `None`, which is a place that is not there yet.
    fn anchor(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        self.anchor_impl(space, pose)
    }

    fn anchor_pose(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        self.anchor_pose_impl(anchor, space)
    }

    fn release_anchor(&mut self, anchor: wxr::Anchor) {
        self.release_anchor_impl(anchor)
    }

    fn anchor_id(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        self.anchor_id_impl(anchor)
    }

    /// Ask for a source of hits along a ray - the app's, or the space's own aim - which is
    /// `XR_META_environment_raycast`.
    ///
    /// The ray is not part of the runtime's source: a cast takes the ray, so a source is the space and the
    /// ray, and `hits` is where it lands. The raycaster is made on the first call, asynchronously, so a
    /// source made a frame ago may still have nothing behind it.
    fn hit_test_source(
        &mut self,
        space: wxr::ReferenceSpace,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_impl(space, ray)
    }

    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        self.hits_impl(source, space, out)
    }

    /// Let a source of hits go, which is the slot the source's ray is measured from.
    ///
    /// Nothing to destroy: one raycaster serves the session and takes the ray with every call, so a source is
    /// only the space and ray it was made from. The entry stays, because the id is its position.
    fn release_hit_test_source(&mut self, source: wxr::HitTestSource) {
        self.release_hit_test_source_impl(source)
    }

    /// What the runtime measured of the room, which is `XR_META_environment_depth`'s depth map.
    ///
    /// OpenXR's depth is a buffer the *runtime* fills and the app reads, acquired once a frame between the
    /// frame being begun and ended. What comes back is the image and what its values mean.
    fn depth(&mut self, view: usize) -> Option<(&Self::Depth, wxr::DepthInfo)> {
        self.depth_impl(view)
    }

    /// What this session's depth sensing is, once it has a depth provider.
    ///
    /// `XR_META_environment_depth` is one measurement the app asked for, at a resolution the runtime picked and
    /// in a buffer the runtime owns - WebXR's `cpu-optimized`, at the platform's own resolution.
    fn depth_sensing(&self) -> Option<wxr::DepthSensing> {
        self.depth_sensing_impl()
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.end_impl(_frame)
    }

    /// Ask the runtime to end the session, which is `xrRequestExitSession`. The `EXITING` state that follows
    /// arrives as [`wxr::Event::ExitRequested`] from `poll` - the same event a system menu produces.
    fn request_exit(&mut self) -> Result<(), wxr::Error> {
        self.session
            .request_exit()
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))
    }
}

impl OpenXrSession {
    /// OpenXR's own session state, which says more than the core's two axes do - the same thing
    /// [`wxr::Session::as_backend`] reaches, kept as a method because a program with an `OpenXrSession` should
    /// not have to go through `Any` to ask.
    pub fn openxr_state(&self) -> xr::SessionState {
        self.openxr_state
    }
}

mod display;
mod events;
mod frames;
mod sources;
mod spaces;
