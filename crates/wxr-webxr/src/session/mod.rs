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

use state::*;
pub use state::{FrameImage, WebXrSession};
use util::*;

use crate::convert::{
    field_of_view, hand_joint, offset_reference_space, reference_space_type, rigid, transform,
    visibility,
};
use crate::sys::{
    DomPointReadOnly, Event, XrBoundedReferenceSpace, XrDepthDataFormat, XrDepthType, XrDepthUsage,
    XrEnvironmentBlendMode, XrEye, XrFrame, XrHandedness, XrInteractionMode, XrLayerLayout,
    XrProjectionLayer, XrReferenceSpace, XrRenderStateInit, XrSession, XrView, XrgpuBinding,
    XrgpuProjectionLayerInit, XrgpuQuadLayerInit,
};
use crate::{depth, hit, input, planes, throws};

impl Drop for WebXrSession {
    /// End the browser's session when this one is dropped.
    ///
    /// A WebXR session belongs to the browser, and without this it keeps it - and the display it holds - until
    /// the page goes away. Dropping the handle is how an app says it is finished, so this is where the browser
    /// is told.
    fn drop(&mut self) {
        if let Some(session) = self.inner.borrow().session.clone() {
            let _ = session.end();
        }
    }
}

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
            located: 0,
            device: Some(device),
            gpu: None,
            image: None,
            // `Unknown` rather than the default, because a session with no binding has no format and saying
            // `Rgba8Srgb` would be a plausible size dressed up as a fact - which is the one thing this
            // backend's `images` has always refused to do.
            meta: wxr::ImageMeta {
                format: wxr::ColorFormat::Unknown,
                ..Default::default()
            },
            depth_range: None,
            layers: Vec::new(),
            layer_images: Vec::new(),
        }
    }

    /// Ask for the binding and the layer, and keep them if the browser gives them.
    ///
    /// Three things have to be true and only one of them is this crate's to arrange: the session has to have
    /// come back with the `webgpu` feature, the device has to have been made from an XR-compatible adapter,
    /// and the browser has to have the binding at all. Any of them missing leaves a session with a head, two
    /// eyes and a clock and no picture - which is what this backend could already say for itself, so it is a
    /// warning rather than a failure.
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
        // The colour format is the runtime's own preference; the depth format is named here, because a
        // projection layer made without one has no depth texture at all and one made with another would hand
        // over depth the renderer's pipeline cannot be attached to.
        let init = XrgpuProjectionLayerInit::new(&binding.get_preferred_color_format());
        init.set_depth_stencil_format(Some(DEPTH_FORMAT_NAME));
        let layer = match throws::create_projection_layer(&binding, &init) {
            Ok(layer) => layer,
            Err(error) => {
                log::warn!("wxr-webxr: the browser would not make a projection layer: {error:?}");
                return;
            }
        };
        self.gpu = Some(Gpu { binding, layer });
        // The render state is handed the projection layer before anything else, because a WebGPU-compatible
        // session with no layer set is a session whose animation frames never arrive at all.
        if let Err(error) = self.present_layers() {
            log::warn!("wxr-webxr: the browser took no layer list: {error}");
        }
        log::info!("wxr-webxr: a projection layer, and with it the frames");
        // A foveation amount asked for before there was a layer goes on now that there is one.
        self.apply_foveation();
    }

    /// Hand the browser the whole layer list: the projection layer first, then every layer the app has made.
    ///
    /// Called again whenever a layer appears or goes away, because the render state is where a layer *is* - a
    /// layer the browser was never told about is a texture nothing composites.
    fn present_layers(&self) -> Result<(), wxr::Error> {
        let Some(gpu) = &self.gpu else {
            return Ok(());
        };
        let session = self.inner.borrow().session.clone();
        let Some(session) = session else {
            return Ok(());
        };
        let state = XrRenderStateInit::new();
        let presented = js_sys::Array::new();
        presented.push(gpu.layer.as_ref());
        for slot in self.layers.iter().flatten() {
            presented.push(slot.layer.as_ref());
        }
        state.set_layers(&presented);
        // Caught, because `updateRenderState` throws during an animation frame and a layer list is handed over
        // between frames: a caller that breaks that rule gets an error rather than a trap.
        throws::update_render_state(&session, &state)
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))
    }

    /// Put the near and far planes on the browser's render state, if there is a session to put them on.
    ///
    /// Called again when a session arrives, which is what lets an app set them while the session is still
    /// `Connecting`: a render state belongs to the browser's session, and there is none until there is one.
    fn apply_depth_range(&self) {
        let (Some((near, far)), Some(session)) =
            (self.depth_range, self.inner.borrow().session.clone())
        else {
            return;
        };
        let state = XrRenderStateInit::new();
        state.set_depth_near(near as f64);
        state.set_depth_far(far as f64);
        if let Err(error) = throws::update_render_state(&session, &state) {
            log::warn!("wxr-webxr: the near and far planes were refused: {error:?}");
        }
    }

    /// Put the foveation amount on the layer, when there is one - a session with no binding has no layer, and
    /// an amount it never asked for is the layer's own default and is left alone.
    fn apply_foveation(&self) {
        let (Some(gpu), Some(foveation)) = (&self.gpu, self.foveation) else {
            return;
        };
        gpu.layer.set_fixed_foveation(Some(foveation));
    }

    /// Ask for the next frame, once there is a session to ask.
    fn request_frame(&mut self) {
        let session = self.inner.borrow().session.clone();
        let Some(session) = session else {
            return;
        };
        let inner = self.inner.clone();
        {
            let mut borrow = inner.borrow_mut();
            if borrow.awaiting {
                return;
            }
            borrow.awaiting = true;
        }
        let callback = Closure::wrap(Box::new(move |time: f64, frame: XrFrame| {
            // Milliseconds from the page's origin, which is the only clock a browser has. This one has arrived,
            // so the next one can be asked for.
            let mut inner = inner.borrow_mut();
            inner.awaiting = false;
            inner.frame = Some((frame, Duration::from_secs_f64(time / 1000.0)));
        }) as Box<dyn FnMut(f64, XrFrame)>);
        // The handle comes back as a number, and a browser that will not give one has a session that is over.
        let _ = session.request_animation_frame(callback.as_ref().unchecked_ref());
        self.callback = Some(callback);
    }
}

impl wxr::Session for WebXrSession {
    /// The browser's `GPUTexture`, and the depth that came with it: a WebXR/WebGPU projection layer gives a
    /// sub-image per view that shares both and differs only in which part of the colour one the view draws
    /// into - so a frame here is one image, two viewports, and two array layers when the layer is stereo.
    type Image = FrameImage;
    /// The depth buffer, as the browser's own `GPUTexture` - which is what `gpu-optimized` depth arrives as,
    /// and only while the frame it was made for is the frame being drawn.
    type Depth = JsValue;

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

    /// What the display shows behind the picture, which WebXR calls `environmentBlendMode`.
    ///
    /// Opaque until there is a session to ask, which is the honest answer for a session that is not here: what a
    /// display does with the world behind it is a fact about the display, and there is none.
    /// What the browser says the depth sensing is - the only backend where this is an app's request rather
    /// than a compositor's buffer, and so the only one that answers with something.
    fn depth_sensing(&self) -> Option<wxr::DepthSensing> {
        self.depth_sensing_impl()
    }

    /// What the browser says, which is the one place this can be `ScreenSpace`.
    fn interaction_mode(&self) -> wxr::InteractionMode {
        self.interaction_mode_impl()
    }

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

    /// The near and far planes, through the browser's own render state.
    ///
    /// Remembered as well as applied, because an app is free to set them while the session is still
    /// `Connecting` - which is the honest order when the planes are the renderer's choice and not the
    /// runtime's.
    fn set_depth_range(&mut self, near: f32, far: f32) {
        self.depth_range = Some((near, far));
        self.apply_depth_range();
    }

    fn images(&self) -> wxr::ImageMeta {
        // Read off the texture the browser handed over, so a session with no binding reports an unknown format
        // and a zero extent rather than a plausible size: a renderer that reads this and draws anyway has been
        // told the truth.
        self.meta
    }

    fn image_count(&self) -> usize {
        // One: both eyes draw into the same texture. Which part of it is a view's business, and a view says so
        // with its viewport and its layer.
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

    /// A buzz on one source, which is the gamepad's own `pulse`.
    ///
    /// The prompt one where there is a choice between this and `play_pcm`: the standard call is what every
    /// browser with an actuator implements, and the draft's waveform is meta's and OpenXR's.
    fn pulse(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        self.pulse_impl(source, intensity, duration)
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
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_impl(space)
    }

    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        self.hits_impl(source, space, out)
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

    fn set_foveation(&mut self, amount: f32) {
        // Clamped, because the specification says a value outside `0..=1` is clamped rather than refused.
        self.foveation = Some(amount.clamp(0.0, 1.0));
        self.apply_foveation();
    }

    fn request_viewport_scale(&mut self, view: usize, scale: Option<f32>) {
        // `None` is ignored, which the specification says so that a recommendation may be passed unchecked.
        // It lands when the browser next answers for this view's viewport, which is the next `views`.
        if let Some(view) = self.frame_views.get(view) {
            view.request_viewport_scale(scale.map(|scale| scale as f64));
        }
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
        self.release_anchor_impl(anchor)
    }

    fn depth(&mut self, view: usize) -> Option<(&Self::Depth, wxr::DepthInfo)> {
        self.depth_image = None;
        // From the binding and not the frame, because this session asked for GPU depth - the one that arrives
        // as something to draw with rather than as bytes to read.
        let gpu = self.gpu.as_ref()?;
        let view = self.frame_views.get(view)?;
        let (image, info) = depth::information(&gpu.binding, view)?;
        self.depth_image = Some(image);
        self.depth_image.as_ref().map(|image| (image, info))
    }

    fn layer(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
    ) -> Result<wxr::Layer, wxr::Error> {
        self.layer_impl(space, shape, pixels)
    }

    fn layer_image(&mut self, layer: wxr::Layer) -> Option<(&Self::Image, wxr::LayerImage)> {
        self.layer_image_impl(layer)
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
}
mod anchors;
mod events;
mod frames;
mod haptics;
mod layers;
mod light;
mod sensing;
mod sources;
mod spaces;
mod world;
