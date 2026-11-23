//! The session the browser handed over, and everything it is asked.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

mod state;
mod util;
mod visibility;

use state::*;
pub use state::{FrameImage, WebXrSession};
use util::*;

use crate::convert::{
    field_of_view, hand_joint, motion, offset_reference_space, reference_space_type, rigid,
    transform, visibility,
};
use crate::sys::{
    DomPointReadOnly, Event, XrBoundedReferenceSpace, XrDepthDataFormat, XrDepthType, XrDepthUsage,
    XrEnvironmentBlendMode, XrEye, XrFrame, XrHand, XrHandedness, XrInteractionMode, XrLayerLayout,
    XrPose, XrProjectionLayer, XrReferenceSpace, XrReferenceSpaceEvent, XrRenderStateInit,
    XrSession, XrSpace, XrView, XrVisibilityMaskChangeEvent, XrgpuBinding,
    XrgpuProjectionLayerInit, XrgpuQuadLayerInit,
};
use crate::{depth, hit, input, planes, throws};

/// Where the session request has got to.
pub(crate) enum Connect {
    Pending,
    Started(XrSession),
    Failed(String),
}

impl WebXrSession {
    pub(crate) fn new(connect: Rc<RefCell<Connect>>, device: wgpu::Device) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner::default())),
            connect,
            callback: None,
            current: None,
            ended: Rc::new(Cell::new(false)),
            on_end: None,
            rate_changed: Rc::new(Cell::new(false)),
            on_rate_change: None,
            on_visibility_mask: None,
            lost: false,
            spaces: Vec::new(),
            state: wxr::State::Connecting,
            visibility: wxr::Visibility::Hidden,
            sources: input::Sources::new(),
            input: None,
            planes: planes::Ids::default(),
            hit_sources: Vec::new(),
            light_probes: Vec::new(),
            depth_image: None,
            foveation: None,
            anchors: Vec::new(),
            frame_views: Vec::new(),
            reset: Rc::new(RefCell::new(VecDeque::new())),
            redraw: Rc::new(RefCell::new(VecDeque::new())),
            visibility_masks: Rc::new(RefCell::new(Vec::new())),
            visibility_mask_changed: Rc::new(RefCell::new(VecDeque::new())),
            device: Some(device),
            gpu: None,
            image: None,
            // `Unknown` rather than the default: a session with no binding has no format, and `Rgba8Srgb`
            // would be a plausible size dressed up as a fact - which this backend's `images` refuses.
            meta: wxr::ImageMeta {
                format: wxr::ColorFormat::Unknown,
                ..Default::default()
            },
            depth_range: None,
            passthrough_obscured: None,
            layers: Vec::new(),
            layer_images: Vec::new(),
        }
    }

    /// Ask for the binding and the layer, and keep them: any missing leaves a head and no picture, a warning.
    fn start_gpu(&mut self, session: &XrSession) {
        if self.gpu.is_some() || !has_feature(session, "webgpu") {
            return;
        }
        let Some(device) = &self.device else {
            return;
        };
        let Some(js_device) = device.as_webgpu() else {
            log::warn!(
                "wxr-webxr: the device is not a browser WebGPU one, so there are no images to give it"
            );
            return;
        };
        let binding = match XrgpuBinding::new(session, js_device.as_ref()) {
            Ok(binding) => binding,
            Err(error) => {
                log::warn!("wxr-webxr: no images: {error:?}");
                return;
            }
        };
        // The colour format is the runtime's preference; the depth format is named here: no depth, no texture.
        let init = XrgpuProjectionLayerInit::new(&binding.get_preferred_color_format());
        init.set_depth_stencil_format(Some(DEPTH_FORMAT_NAME));
        let layer = match throws::create_projection_layer(&binding, &init) {
            Ok(layer) => layer,
            Err(error) => {
                log::warn!("wxr-webxr: the browser would not make a projection layer: {error:?}");
                return;
            }
        };
        sensing::note_ignored_depth(&layer);
        self.gpu = Some(Gpu { binding, layer });
        // The render state gets the projection layer first: a WebGPU session with no layer set never ticks.
        if let Err(error) = self.present_layers() {
            log::warn!("wxr-webxr: the browser took no layer list: {error}");
        }
        log::info!("wxr-webxr: a projection layer, and with it the frames");
        // A foveation amount asked for before there was a layer goes on now that there is one.
        self.apply_foveation();
    }
}

impl wxr::Session for WebXrSession {
    /// The browser's `GPUTexture` and the depth that came with it: a sub-image per view shares both.
    type Image = FrameImage;
    /// What the browser measured of the room, as its own `GPUTexture`, while its frame is being drawn.
    type EnvironmentDepth = JsValue;

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

    fn poll(&mut self) -> Option<wxr::Event> {
        self.poll_impl()
    }

    /// What the browser says the depth sensing is: an app's request, and the only backend that answers one.
    fn depth_sensing(&self) -> Option<wxr::DepthSensing> {
        self.depth_sensing_impl()
    }

    /// Start or stop the browser measuring the room, the one backend with a depth sensor an app can switch.
    fn set_depth_active(&mut self, active: bool) -> Result<(), wxr::Error> {
        self.set_depth_active_impl(active)
    }

    /// What the browser says, which is the one place this can be `ScreenSpace`.
    fn interaction_mode(&self) -> wxr::InteractionMode {
        self.interaction_mode_impl()
    }

    /// What the display shows behind the picture - WebXR's `environmentBlendMode` - `Opaque` until asked.
    fn blend(&self) -> wxr::Blend {
        let Some(session) = self.inner.borrow().session.clone() else {
            return wxr::Blend::Opaque;
        };
        match session.environment_blend_mode() {
            XrEnvironmentBlendMode::Additive => wxr::Blend::Additive,
            XrEnvironmentBlendMode::AlphaBlend => wxr::Blend::AlphaBlend,
            XrEnvironmentBlendMode::Opaque => wxr::Blend::Opaque,
            // A value from a newer specification than these bindings. Opaque is the conservative answer: it
            // says nothing about the world behind the picture, which is what not knowing means.
            XrEnvironmentBlendMode::__Invalid => wxr::Blend::Opaque,
        }
    }

    /// The near and far planes, through the browser's render state; remembered for one set while `Connecting`.
    fn set_depth_range(&mut self, range: wxr::DepthRange) {
        self.depth_range = Some(range);
        self.apply_depth_range();
    }

    /// A promise that the picture covers the camera feed, through the same render state; remembered while
    /// `Connecting`, because there is no state to put it on until the session arrives.
    fn set_passthrough_fully_obscured(&mut self, obscured: bool) {
        self.passthrough_obscured = Some(obscured);
        self.apply_passthrough_obscured();
    }

    fn images(&self) -> wxr::ImageMeta {
        // Read off the texture, so a session with no binding reports an unknown format and a zero extent.
        self.meta
    }

    fn image_count(&self) -> usize {
        // One: both eyes draw into the same texture, and which part is a view's business.
        usize::from(self.image.is_some())
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        (index == 0).then_some(self.image.as_ref()).flatten()
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        self.space_impl(kind)
    }

    fn offset_space(
        &mut self,
        space: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        self.offset_space_impl(space, offset)
    }

    fn locate(
        &mut self,
        space: wxr::ReferenceSpace,
        base: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::TrackedPose>, wxr::Error> {
        self.locate_impl(space, base)
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.begin_impl(_now, out)
    }

    fn binding(&mut self, _layer: wxr::Layer) -> Result<wxr::Binding, wxr::Error> {
        self.binding_impl(_layer)
    }

    fn sub_image(&mut self, _binding: wxr::Binding, view: usize) -> Option<wxr::SubImage> {
        self.sub_image_impl(_binding, view)
    }

    fn views(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        self.views_impl(space, out)
    }

    /// What part of a view the display shows, or `Unsupported` when the browser never exposed the attribute.
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

    /// A source's profiles, which is `XRInputSource.profiles` copied as it stands.
    fn profiles(&mut self, source: wxr::InputId, out: &mut Vec<String>) -> Result<(), wxr::Error> {
        self.profiles_impl(source, out)
    }

    /// A buzz on one source, which is the gamepad's own `pulse` rather than the draft's `play_pcm`.
    fn pulse(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        self.pulse_impl(source, intensity, duration)
    }

    fn stop_pulse(&mut self, source: wxr::InputId) -> Result<(), wxr::Error> {
        self.stop_impl(source)
    }

    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        self.planes_impl(space, out)
    }

    fn bounds(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        self.bounds_impl(space, out)
    }

    fn hit_test_source(
        &mut self,
        space: wxr::ReferenceSpace,
        kinds: wxr::HitTestKinds,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_impl(space, kinds, ray)
    }

    fn hit_test_source_for_profile(
        &mut self,
        kinds: wxr::HitTestKinds,
        profile: &str,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_for_profile_impl(kinds, profile)
    }

    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        self.hits_impl(source, space, out)
    }

    fn release_hit_test_source(&mut self, source: wxr::HitTestSource) {
        self.release_hit_test_source_impl(source)
    }

    fn light_probe(&mut self) -> Result<wxr::LightProbe, wxr::Error> {
        self.light_probe_impl()
    }

    fn light(
        &mut self,
        probe: wxr::LightProbe,
        out: &mut wxr::LightEstimate,
    ) -> Result<(), wxr::Error> {
        self.light_impl(probe, out)
    }

    fn light_probe_pose(
        &mut self,
        probe: wxr::LightProbe,
        space: wxr::ReferenceSpace,
    ) -> Option<wxr::Pose> {
        self.light_probe_pose_impl(probe, space)
    }

    fn set_foveation(&mut self, amount: f32) {
        // `fixedFoveation` is a WebIDL `float`, so NaN and the infinities are a `TypeError` - an exception
        // through the wasm frame, not a knob refusing a value. NaN is refused here, and `clamp` tames infinity.
        if amount.is_nan() {
            log::debug!("wxr-webxr: no foveation for {amount}, which is not a number");
            return;
        }
        // Clamped, because the specification says a value outside `0..=1` is clamped rather than refused.
        self.foveation = Some(amount.clamp(0.0, 1.0));
        self.apply_foveation();
    }

    fn refresh_rates(&mut self, out: &mut Vec<f32>) {
        self.refresh_rates_impl(out);
    }

    fn refresh_rate(&mut self) -> Option<f32> {
        self.refresh_rate_impl()
    }

    fn set_refresh_rate(&mut self, rate: f32) {
        self.set_refresh_rate_impl(rate);
    }

    fn request_viewport_scale(&mut self, view: usize, scale: Option<f32>) {
        self.request_viewport_scale_impl(view, scale)
    }

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
        self.release_anchor_impl(anchor);
    }

    fn anchor_id(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        self.anchor_id_impl(anchor)
    }

    fn restore_anchor(&mut self, id: wxr::AnchorId) -> Result<wxr::Anchor, wxr::Error> {
        self.restore_anchor_impl(id)
    }

    fn persistent_anchor_ids(&mut self, out: &mut Vec<wxr::AnchorId>) -> Result<(), wxr::Error> {
        self.persistent_anchor_ids_impl(out)
    }

    fn delete_persistent_anchor(&mut self, id: wxr::AnchorId) -> Result<(), wxr::Error> {
        self.delete_persistent_anchor_impl(id)
    }

    fn environment_depth(
        &mut self,
        view: usize,
    ) -> Option<(&Self::EnvironmentDepth, wxr::DepthInfo)> {
        self.depth_image = None;
        // A runtime that has paused depth has nothing for this frame, even if the binding would still answer.
        if self.depth_paused() {
            return None;
        }
        // From the binding, not the frame: this session asked for GPU depth, which arrives to draw with.
        let gpu = self.gpu.as_ref()?;
        let view = self.frame_views.get(view)?;
        let (image, info) = depth::information(&gpu.binding, view)?;
        self.depth_image = Some(image);
        self.depth_image.as_ref().map(|image| (image, info))
    }

    fn max_render_layers(&self) -> Option<u32> {
        self.max_render_layers_impl()
    }

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

    fn layer_needs_redraw(&self, layer: wxr::Layer) -> bool {
        self.layer_needs_redraw_impl(layer)
    }

    fn set_layer_pose(&mut self, layer: wxr::Layer, pose: wxr::Pose) -> Result<(), wxr::Error> {
        self.set_layer_pose_impl(layer, pose)
    }

    fn release_layer(&mut self, layer: wxr::Layer) {
        self.release_layer_impl(layer)
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.end_impl(_frame)
    }

    fn request_exit(&mut self) -> Result<(), wxr::Error> {
        self.request_exit_impl()
    }
}
mod anchors;
mod events;
mod exit;
mod frames;
mod haptics;
mod layers;
mod light;
mod sensing;
mod sources;
mod spaces;
mod world;
