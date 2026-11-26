//! The session the runtime handed over, and everything it is asked.

use std::ffi::c_void;
use std::time::Duration;

use openxr as xr;

mod anchors;
mod depth;
mod depth_submission;
mod hits;
mod layers;
mod persistence;
mod planes;
mod state;
mod visibility;

// The types the world-modelling modules make, so `state` can hold them and the trait block below can point
// at them. `EnvironmentDepthImage` is public because it is this backend's `Session::EnvironmentDepth`.
use anchors::Anchors;
use depth::{EnvironmentDepth, MeasuredDepth};
use hits::Raycaster;
use planes::PlaneDetector;
use visibility::VisibilityMasks;

pub(crate) use state::{HapticExtensions, InputExtensions, LayerExtensions, SpatialExtensions};

pub use depth::EnvironmentDepthImage;
use spaces::{is_located, resets_for};
pub use state::OpenXrSession;
use state::*;

use crate::convert::{color_format, field_of_view, pose, posef, reference_space};
use crate::{Device, Error, OpenXr, hal, input};

mod making;

impl wxr::Session for OpenXrSession {
    /// The colour image this frame drew into and the depth image it drew with, when the runtime took one.
    type Image = crate::import::SwapchainImage;
    /// The runtime's own depth measurement for this frame, from `XR_META_environment_depth`.
    type EnvironmentDepth = EnvironmentDepthImage;

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

    /// The rates the runtime offers, which is `xrEnumerateDisplayRefreshRatesFB`; none without the extension.
    fn refresh_rates(&mut self, out: &mut Vec<f32>) {
        self.refresh_rates_impl(out)
    }

    /// The rate the display is running at, which is `xrGetDisplayRefreshRateFB`; `None` without the extension.
    fn refresh_rate(&mut self) -> Option<f32> {
        self.refresh_rate_impl()
    }

    /// Ask for one of them, which is `xrRequestDisplayRefreshRateFB`. A refusal is logged, not returned.
    fn set_refresh_rate(&mut self, rate: f32) {
        self.set_refresh_rate_impl(rate)
    }

    /// Ask for app space warp; the frame that carries it also has to carry motion vectors, which
    /// `Import::target_motion` is how the app draws.
    fn set_space_warp(&mut self, enabled: bool) {
        self.space_warp = enabled;
    }

    fn set_foveation(&mut self, amount: f32) {
        self.set_foveation_impl(amount)
    }

    /// The near and far planes the scene draws with, from `XrCompositionLayerDepthInfoKHR`; saying it turns depth on.
    fn set_depth_range(&mut self, range: wxr::DepthRange) {
        self.depth_range = Some(range);
    }

    /// The pair this session will put in `XrCompositionLayerDepthInfoKHR` - the app's own, because saying it
    /// is also what turns the depth submission on, so there is no runtime default to learn. `None` until the
    /// app has said.
    fn depth_range(&self) -> Option<wxr::DepthRange> {
        self.depth_range
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
        // The pair this frame acquired, which the compositor will present - not the first of either
        // swapchain's, which are different images on most frames. `held` is what says a frame is open.
        self.held?;
        (index == 0).then_some(self.frame_image.as_ref()).flatten()
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

    fn locate(
        &mut self,
        space: wxr::ReferenceSpace,
        base: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::TrackedPose>, wxr::Error> {
        self.locate_impl(space, base)
    }

    fn bounds(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        self.bounds_impl(space, out)
    }

    /// Make a picture the compositor places itself, which is `xrCreateSwapchain` for the shape's extension.
    fn layer(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
        options: wxr::LayerOptions,
    ) -> Result<wxr::Layer, wxr::Error> {
        self.layer_impl(space, shape, pixels, options)
    }

    fn layer_image(&mut self, layer: wxr::Layer) -> Option<(&Self::Image, wxr::LayerImage)> {
        self.layer_image_impl(layer)
    }

    fn set_layer_pose(&mut self, layer: wxr::Layer, pose: wxr::Pose) -> Result<(), wxr::Error> {
        self.set_layer_pose_impl(layer, pose)
    }

    /// Dropping the swapchain is the whole of it: OpenXR has no `destroy`, and a layer no longer named is gone.
    fn release_layer(&mut self, layer: wxr::Layer) {
        self.release_layer_impl(layer)
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

    /// What part of a view the display shows, or `Unsupported` without `XR_KHR_visibility_mask`.
    fn visibility_mask(
        &mut self,
        view: usize,
        out: &mut wxr::VisibilityMask,
    ) -> Result<(), wxr::Error> {
        self.visibility_mask_impl(view, out)
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

    /// The runtime's name for a source in the user's language, which is `xrGetInputSourceLocalizedName`.
    fn input_source_name(&self, source: wxr::InputId) -> Option<String> {
        self.input_source_name_impl(source)
    }

    /// A vibration on one source, which is `xrApplyHapticFeedback`. A source with no output path - and every
    /// source without the extension - is `Unsupported` rather than a silent nothing: the app chose to call.
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

    /// The surfaces the runtime has found this frame, which is `XR_EXT_plane_detection`'s state machine: one
    /// call is one *step* of begin, poll and read, so an empty list is a pass that has not finished.
    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        self.planes_impl(space, out)
    }

    /// Ask the runtime to keep a place fixed in the room, which is `XR_FB_spatial_entity`'s anchor. Made
    /// asynchronously: `anchor_pose` on one that has not arrived is `None`.
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

    fn persistent_anchor_ids(&mut self, out: &mut Vec<wxr::AnchorId>) -> Result<(), wxr::Error> {
        self.persistent_anchor_ids_impl(out)
    }

    /// Find a place the runtime stored - `XR_META_spatial_entity_discovery`.
    fn restore_anchor(&mut self, id: wxr::AnchorId) -> Result<wxr::Anchor, wxr::Error> {
        self.restore_anchor_impl(id)
    }

    /// Forget a name for good, which `XR_META_spatial_entity_persistence` takes by UUID.
    fn delete_persistent_anchor(&mut self, id: wxr::AnchorId) -> Result<(), wxr::Error> {
        self.delete_persistent_anchor_impl(id)
    }

    /// A ray into the room, which is `XR_META_environment_raycast`; a narrowing is `Unsupported`.
    fn hit_test_source(
        &mut self,
        space: wxr::ReferenceSpace,
        kinds: wxr::HitTestKinds,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_impl(space, kinds, ray)
    }

    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        self.hits_impl(source, space, out)
    }

    /// Let a source of hits go. One raycaster serves the session, so a source is only a space and a ray.
    fn release_hit_test_source(&mut self, source: wxr::HitTestSource) {
        self.release_hit_test_source_impl(source)
    }

    /// What the runtime measured of the room: `XR_META_environment_depth`'s buffer, acquired once a frame.
    fn environment_depth(
        &mut self,
        view: usize,
    ) -> Option<(&Self::EnvironmentDepth, wxr::DepthInfo)> {
        self.environment_depth_impl(view)
    }

    /// What this session's depth sensing is: one measurement in a runtime-owned buffer, at its resolution.
    fn depth_sensing(&self) -> Option<wxr::DepthSensing> {
        self.depth_sensing_impl()
    }

    /// Start or stop the runtime measuring the room, which is `XR_META_environment_depth`'s start and stop.
    fn set_depth_active(&mut self, active: bool) -> Result<(), wxr::Error> {
        self.set_depth_active_impl(active)
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

mod display;
mod events;
mod frames;
mod sources;
mod space_warp;
mod spaces;
mod submit;
