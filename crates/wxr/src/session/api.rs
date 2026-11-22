//! What a session says, which is the whole of the core.
//!
//! In a file of its own because it is the side of the seam that grows. The images are an associated type and
//! not a handle, because the three platforms do not agree on what one is, and [`Any`] is there for the same
//! kind of reason: the core is a subset, and a subset has to be able to hand back what it left out.

use std::any::Any;
use std::time::Duration;

use glam::Vec2;

use crate::anchor::{Anchor, AnchorId};
use crate::authorization::Authorization;
use crate::depth::{DepthInfo, DepthSensing};
use crate::device_fit::DeviceFit;
use crate::feature::Features;
use crate::frame::{DepthRange, Frame};
use crate::hit::{Hit, HitTestKinds, HitTestSource, Ray};
use crate::input::{Hand, InputId, InputSource};
use crate::layer::{Binding, Layer, LayerImage, LayerOptions, LayerShape, SubImage};
use crate::light::{LightEstimate, LightProbe};
use crate::mesh::Mesh;
use crate::plane::Plane;
use crate::space::{Pose, ReferenceSpace, SpaceKind, TrackedPose};
use crate::target::ImageMeta;
use crate::visibility::VisibilityMask;

use super::*;

/// A live session, as a backend fills it in and a caller is written against.
pub trait Session: Any {
    /// The compositor's own name for an image. Opaque here, on purpose.
    type Image;

    /// The runtime's own name for the depth it measured of the room, when it measured any - a browser
    /// texture, a Vulkan image. Named for the environment because the other depth a session has is the
    /// attachment an app *draws into*, which arrives with the image and needs no type of its own. An
    /// associated type like [`Session::Image`], and `()` for a session that measures nothing.
    type EnvironmentDepth;

    /// The backend's own session, when a program needs the platform the core deliberately does not speak -
    /// the core's `as_hal`; the module docs say why, and what a session held as a trait object loses.
    fn as_backend<T: 'static>(&self) -> Option<&T>
    where
        Self: Sized,
    {
        (self as &dyn Any).downcast_ref()
    }

    /// The same, mutably, for the part of a platform a program sets rather than reads.
    fn as_backend_mut<T: 'static>(&mut self) -> Option<&mut T>
    where
        Self: Sized,
    {
        (self as &mut dyn Any).downcast_mut()
    }

    fn presentation(&self) -> Presentation;

    /// What this session can do beyond what every session can, which is [`Features`] - what the backend
    /// actually got and not what it asked for, since a browser may refuse a feature and a runtime lack one.
    fn features(&self) -> Features;

    /// Whether this session is allowed to use `feature`, which is what tells "not on this device" from "you
    /// said no" - [`features`](Session::features) says what it has, and this says whether it was allowed it.
    fn authorization(&mut self, _feature: Features) -> Authorization {
        Authorization::Granted
    }

    /// The session's lifecycle: is it being asked for, is it here, is it over.
    fn state(&self) -> State;

    /// Whether the session is being shown. Hidden by default: a backend that does not say has a display nobody
    /// has looked at, and drawing into an unshown session draws into nothing.
    fn visibility(&self) -> Visibility {
        Visibility::Hidden
    }

    /// Where the wearer's eyes sit in the display's range - see [`DeviceFit`]. [`Features::DEVICE_FIT`] says
    /// whether this session can answer; `Unknown` is both "cannot" and "not measured yet".
    fn device_fit(&mut self) -> DeviceFit {
        DeviceFit::Unknown
    }

    /// The next thing that happened, or `None` if nothing has. Drained one at a time, because two events in a
    /// row can matter in order - `Visible` then `ExitRequested` is a session that ran and stopped.
    fn poll(&mut self) -> Option<Event>;

    /// What the display shows behind the picture. `Opaque` by default, because a session that does not say is
    /// a session that fills the display - which is what both ways of presenting into one do.
    fn blend(&self) -> Blend {
        Blend::Opaque
    }

    /// What this session's depth sensing is, when it asked for any - WebXR's `depthUsage`, `depthDataFormat`,
    /// `depthType` and `depthActive` as one value. `None` for no depth sensing and for a compositor's own depth.
    fn depth_sensing(&self) -> Option<DepthSensing> {
        None
    }

    /// Start or stop the runtime measuring the room - WebXR's `resumeDepthSensing` and `pauseDepthSensing`.
    /// A request: no depth to pause is [`Error::Unsupported`], and the state already reported is a no-op.
    fn set_depth_active(&mut self, _active: bool) -> Result<(), Error> {
        Err(Error::Unsupported("pausing depth sensing".into()))
    }

    /// How this session reads its input - WebXR's `interactionMode`. `WorldSpace` by default, because that is
    /// what a session drawn into a display is; a browser's can be `ScreenSpace` too, so that backend asks.
    fn interaction_mode(&self) -> InteractionMode {
        InteractionMode::WorldSpace
    }

    /// The near and far planes the scene draws with, in metres - [`DepthRange`] says why a compositor needs
    /// them and why they are one value. A pair that does not describe a projection reaches the runtime as is.
    fn set_depth_range(&mut self, _range: DepthRange) {}

    /// How much foveation the compositor should apply, from `0` to `1` - WebXR's `fixedFoveation`. A request,
    /// so a compositor that does not foveate ignores it; nothing is sent by default.
    fn set_foveation(&mut self, _amount: f32) {}

    /// The refresh rates the display offers, in hertz, appended to `out` - `XR_FB_display_refresh_rate`'s
    /// list. Empty by default: a session whose platform does not let an app choose has no list to offer.
    fn refresh_rates(&mut self, _out: &mut Vec<f32>) {}

    /// The rate the display is running at in hertz, or `None` - WebXR's `XRSession.frameRate`, OpenXR's
    /// `xrGetDisplayRefreshRateFB`. It lags a request, so this is how a caller learns whether one took.
    fn refresh_rate(&mut self) -> Option<f32> {
        None
    }

    /// Ask the display to run at `rate` hertz - the other half of [`refresh_rates`](Session::refresh_rates).
    /// A *request*, like [`set_foveation`](Session::set_foveation): a runtime that cannot change its rate, or
    /// will not take this one, keeps the rate it has - and that default is the display's own choice.
    fn set_refresh_rate(&mut self, _rate: f32) {}

    /// Ask for each frame to be shown `count` more times before the next one - CompositorServices'
    /// `cp_layer_renderer_minimum_frame_repeat_count`, where the frame rate is not the app's to set. Zero is
    /// every frame, one halves the rate, and a platform that cannot repeat anything ignores the request.
    fn set_frame_repeat(&mut self, _count: u32) {}

    /// Ask for a view to be rendered at `scale` of its full resolution - WebXR's `requestViewportScale`, the
    /// app's half of the foveation trade. `None` is ignored, so passing a `recommended_viewport_scale` is safe.
    fn request_viewport_scale(&mut self, _view: usize, _scale: Option<f32>) {}

    /// Ask the compositor to synthesise intermediate frames from our own motion vectors and depth - Meta's
    /// `XR_FB_space_warp`. A request with two sides: it is only honest when the app draws motion vectors into
    /// the image the session hands it (see `wxr_render::Import::target_motion`) and submits depth, because
    /// otherwise the compositor synthesises from an image nobody wrote. False - the default - leaves every
    /// frame as it is; a runtime without the extension ignores the request.
    fn set_space_warp(&mut self, _enabled: bool) {}

    /// Ask the compositor to draw at a share of its full resolution, `0` to `1` - visionOS's
    /// `cp_layer_renderer_set_render_quality`, a whole-session knob rather than a per-view one. A request.
    fn set_render_quality(&mut self, _quality: f32) {}

    /// The shape of every image this session presents.
    fn images(&self) -> ImageMeta;

    /// How many images there are: one per eye for a compositor that does not layer them, one when it does.
    fn image_count(&self) -> usize;

    /// The image at `index`, as this platform names it.
    fn image(&self, index: usize) -> Option<&Self::Image>;

    /// How many layers this session may hand the compositor at once, when the runtime names a number -
    /// WebXR's `maxRenderLayers`. `None` is not zero: an unpublished limit is asked by making a layer.
    fn max_render_layers(&self) -> Option<u32> {
        None
    }

    /// Make a picture the compositor places itself - a WebXR or OpenXR `XRCompositionLayer` other than the
    /// projection one. It is drawn once (a menu, a video, a 360° photograph) and the runtime puts it where the
    /// shape, the space, `pixels` and `options` say - see [`LayerOptions`]. Which shapes a session has is
    /// [`Features::LAYER_QUAD`] and its siblings, one bit per shape; a session with none says `Unsupported`.
    fn layer(
        &mut self,
        _space: ReferenceSpace,
        _shape: LayerShape,
        _pixels: crate::Extent2d,
        _options: LayerOptions,
    ) -> Result<Layer, Error> {
        Err(Error::Unsupported("layers".into()))
    }

    /// What to draw into for a layer this frame, and where in it. `None` while there is nothing to draw into
    /// yet - a layer is made asynchronously, and its image is the compositor's for one frame and not the app's
    /// to keep - and for a backend never taught layers either.
    fn layer_image(&mut self, _layer: Layer) -> Option<(&Self::Image, LayerImage)> {
        None
    }

    /// Whether the compositor wants this layer drawn again, which is WebXR's `needsRedraw`. True for a layer
    /// nothing has been drawn into yet and after a [`Redraw`](Event::Redraw) that says its picture was lost;
    /// false once the app has taken the layer's image, and for a compositor given a complete list each frame.
    fn layer_needs_redraw(&self, _layer: Layer) -> bool {
        false
    }

    /// A binding for a layer whose picture the app supplies, which is WebXR's `XRWebGLBinding` and its
    /// WebGPU and media siblings collapsed into one - see [`Binding`]. Not a [`Features`] bit: it is a layer
    /// the app draws into itself, and a backend that cannot take one says [`Error::Unsupported`].
    fn binding(&mut self, _layer: Layer) -> Result<Binding, Error> {
        Err(Error::Unsupported("bindings".into()))
    }

    /// The part of a bound layer's image one eye draws into, which is WebXR's `getSubImage` - the sizes,
    /// the viewport and the array slice. `None` when the binding is not one this session made.
    fn sub_image(&mut self, _binding: Binding, _view: usize) -> Option<SubImage> {
        None
    }

    /// Put a layer where it is, in the space it was made in - WebXR's `XRCompositionLayer.transform`, OpenXR's
    /// layer pose. Per frame for one that moves and once for one that does not.
    fn set_layer_pose(&mut self, _layer: Layer, _pose: Pose) -> Result<(), Error> {
        Err(Error::Unsupported("layers".into()))
    }

    /// Tell the runtime the app is done with a layer. Nothing by default: a backend with no layers has none to
    /// release, and unlike an anchor a layer holds an image the runtime wants back - a reason to say so.
    fn release_layer(&mut self, _layer: Layer) {}

    /// Ask for a space to measure poses in. A runtime that does not have the one asked for says so, rather
    /// than quietly handing back a worse one - a scene that asked to stand on the floor and got a head
    /// origin is a scene floating at eye height.
    fn space(&mut self, kind: SpaceKind) -> Result<ReferenceSpace, Error>;

    /// A space at `offset` inside `space`, for content that belongs to a place rather than to a room: WebXR's
    /// `getOffsetReferenceSpace`, which is how a panel goes on a wall. The offset is in `space`, so what comes
    /// back moves with it - off a controller follows the controller, off the floor stays put.
    fn offset_space(
        &mut self,
        space: ReferenceSpace,
        offset: Pose,
    ) -> Result<ReferenceSpace, Error>;

    /// Where `space` is seen from `base`, which is WebXR's `XRFrame.getPose` and OpenXR's `xrLocateSpace`.
    /// The other direction from [`views`](Session::views): where one space sits inside another. `Ok(None)`
    /// is a space the runtime cannot place right now; a handle this session never gave out is [`Error::NoSpace`].
    fn locate(
        &mut self,
        space: ReferenceSpace,
        base: ReferenceSpace,
    ) -> Result<Option<TrackedPose>, Error>;

    /// Begin a frame. Fills in the predicted display time and what the caller should do with it.
    fn begin(&mut self, now: Duration, out: &mut Frame) -> Result<(), Error>;

    /// Fill in the views for the frame just begun, in the space given. Replaced rather than added to, as
    /// [`Frame::reset`] says - asking twice is two answers, and a call that cannot locate leaves neither.
    fn views(&mut self, space: ReferenceSpace, out: &mut Frame) -> Result<(), Error>;

    /// Fill in the part of one view's picture the display shows, and let an empty mask mean the whole view.
    /// `view` indexes the frame's views, which is what [`Event::VisibilityMaskChanged`] carries; the mask is in
    /// that eye's view space on the plane `z = -1`, as [VisibilityMask] describes. A session without
    /// [`Features::VISIBILITY_MASK`] says [`Error::Unsupported`] rather than a mask claiming the whole view.
    fn visibility_mask(&mut self, _view: usize, out: &mut VisibilityMask) -> Result<(), Error> {
        out.clear();
        Err(Error::Unsupported("visibility masks".into()))
    }

    /// Fill in what the user is holding, in the space given. Emptied first, so what it holds is this frame's
    /// sources rather than the session's history; empty by default, because a runtime with no controllers bound
    /// has nothing to report rather than having failed.
    fn inputs(&mut self, _space: ReferenceSpace, out: &mut Vec<InputSource>) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Fill in a hand's skeleton, if the source has one and the runtime is tracking it.
    ///
    /// WebXR's `XRFrame.getJointPose`, asked once for the whole hand rather than once per joint. A source
    /// with no skeleton says so with `InputSource::hand`, and a hand half out of view has joints that are
    /// not tracked: both are an empty [`Hand`].
    fn hand(
        &mut self,
        _source: InputId,
        _space: ReferenceSpace,
        out: &mut Hand,
    ) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// The runtime's own name, or names, for what a source is - WebXR's `XRInputSource.profiles`.
    ///
    /// The first is the profile in use and the rest are fallbacks, most specific first. The strings belong to
    /// the runtime: WebXR uses its registry's names (`"meta-quest-touch-plus"`), OpenXR the path
    /// `xrGetCurrentInteractionProfile` answers with, and a platform that only names a product names that.
    ///
    /// Empty when the session never handed the source out, has no name for it, or cannot ask - the same "no
    /// name" the other list-shaped answers give, and what to check before drawing a model of a controller.
    fn profiles(&mut self, _source: InputId, out: &mut Vec<String>) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Make a source buzz: `intensity` from `0` to `1`, for `duration` - WebXR's `GamepadHapticActuator.pulse`,
    /// and the one shape every platform with an actuator can be asked for. A *request*, like
    /// [`set_foveation`](Session::set_foveation): intensity is clamped rather than refused, and a platform
    /// that cannot reach its hardware says [`Error::Unsupported`]. [`InputSource::haptics`] is the cheaper
    /// question, and [`Features::HAPTICS`] says whether there is anything to ask at all.
    fn pulse(
        &mut self,
        _source: InputId,
        _intensity: f32,
        _duration: Duration,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("haptics".into()))
    }

    /// Stop a source's vibration before its `duration` runs out - WebXR's `GamepadHapticActuator.reset` and
    /// OpenXR's `xrStopHapticFeedback`.
    ///
    /// The other half of [`pulse`](Session::pulse), and a call of its own because a duration is a guess: the app
    /// learns it was wrong - the level ended, the hand let go. An id never handed out and a source with no
    /// actuator answer [`Error::Unsupported`], as [`pulse`](Session::pulse) does; one not buzzing is `Ok(())`.
    fn stop_pulse(&mut self, _source: InputId) -> Result<(), Error> {
        Err(Error::Unsupported("haptics".into()))
    }

    /// Play a buffer of haptic samples on a source - WebXR's `GamepadHapticActuator.playPCM` and OpenXR's
    /// `XR_FB_haptic_pcm`, which are the same idea: the app hands over the waveform instead of describing it
    /// with an intensity.
    ///
    /// `samples` are amplitudes in `-1..=1` at `sample_rate` hertz, which is what a platform's PCM haptics
    /// want and what a browser's `AudioBuffer` carries, so an app that already has an audio buffer has the
    /// hard part. A backend without PCM output answers [`Error::Unsupported`], and a source that cannot be
    /// felt is the same quiet no-op [`pulse`](Session::pulse) makes of one. Not every session has this even
    /// where it has a pulse: OpenXR's PCM is Meta's extension on top of the vibration one, so a runtime can
    /// have the buzz and not the waveform.
    fn play_pcm(
        &mut self,
        _source: InputId,
        _samples: &[f32],
        _sample_rate: f32,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("haptic samples".into()))
    }

    /// Fill in the surfaces the runtime has found, in the space given.
    ///
    /// WebXR's `XRFrame.detectedPlanes`, the shape a frame's views have: what is detected *this* frame, not a
    /// list to keep. Empty for a session that found none, and for a backend not taught surfaces.
    fn planes(&mut self, _space: ReferenceSpace, out: &mut Vec<Plane>) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Fill in the room the runtime has traced, as triangles, in the space given - WebXR's meshing draft.
    ///
    /// What is traced *this* frame, because a runtime re-traces a room as it learns more and a copy a caller
    /// keeps goes wrong where it matters - a wall that moved. Empty for a session that traced nothing and for
    /// a backend never taught it, the same answer to a caller asking whether there is a wall here.
    ///
    /// A mesh is per *geometry* and not per anchor: one anchor can carry a table and the wall behind it.
    /// Whether a session has this at all is [`Features::MESH`], not [`Features::PLANES`] - a runtime can
    /// trace meshes and have no planes, and ARKit's are two providers.
    fn meshes(&mut self, _space: ReferenceSpace, out: &mut Vec<Mesh>) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Fill in the boundary of a bounded space, as the outline the runtime knows - WebXR's
    /// `XRBoundedReferenceSpace.boundsGeometry`. Points in the space's own X and Z, in metres, around a closed
    /// loop. Empty for a space with no boundary and a backend that never had one, which is not an error.
    fn bounds(&mut self, _space: ReferenceSpace, out: &mut Vec<Vec2>) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Ask for a source of hits along a ray - WebXR's `requestHitTestSource`. `ray` is in `space`'s own
    /// coordinates and `None` means its `-Z`; `kinds` is what it may land on, and kinds this backend cannot
    /// produce are [`Error::Unsupported`] rather than a source that never answers - so [`HitTestKinds::ALL`]
    /// and [`HitTestKinds::NONE`] always work. Made asynchronously: a frame or two with none, and `hits` on
    /// one not ready has none. A ray [`Ray::unit`] refuses is refused too.
    fn hit_test_source(
        &mut self,
        _space: ReferenceSpace,
        _kinds: HitTestKinds,
        _ray: Option<Ray>,
    ) -> Result<HitTestSource, Error> {
        Err(Error::Unsupported("hit testing".into()))
    }

    /// Ask for a source of hits along whichever input source carries `profile` - WebXR's
    /// `requestHitTestSourceForTransientInput`; [`Hit::input`] names the source each hit came from.
    fn hit_test_source_for_profile(
        &mut self,
        _kinds: HitTestKinds,
        _profile: &str,
    ) -> Result<HitTestSource, Error> {
        Err(Error::Unsupported("input hit testing".into()))
    }

    /// Fill in what the ray met this frame, expressed in the space given. Empty when it met nothing; a hit from
    /// a source asked for by profile carries the input that found it.
    fn hits(
        &mut self,
        _source: HitTestSource,
        _space: ReferenceSpace,
        out: &mut Vec<Hit>,
    ) -> Result<(), Error> {
        out.clear();
        Ok(())
    }

    /// Let a source of hits go, which is WebXR's `XRHitTestSource.cancel`; `hits` on a released one is an
    /// empty list rather than an error. WebXR has a `cancel()` and a promise in flight to cancel, while
    /// OpenXR's raycaster has no per-source object - only this backend's slot to clear.
    fn release_hit_test_source(&mut self, _source: HitTestSource) {}

    /// Ask for a probe of the room's light, which is WebXR's `requestLightProbe`. A handle the way a hit-test
    /// source is, and for the same reason: a runtime makes one asynchronously. The estimate it gives is in the
    /// runtime's own probe space, which is not one of this session's - see [`LightEstimate`].
    fn light_probe(&mut self) -> Result<LightProbe, Error> {
        Err(Error::Unsupported("light estimation".into()))
    }

    /// Fill in what the room's light is this frame, from a probe. No intensity and zero harmonics with no
    /// estimate, which is a scene lit by nothing rather than by a guess.
    fn light(&mut self, _probe: LightProbe, out: &mut LightEstimate) -> Result<(), Error> {
        *out = LightEstimate::default();
        Ok(())
    }

    /// Ask the runtime to keep a place fixed in the room, which is WebXR's `XRFrame.createAnchor`: `space`
    /// and `pose` are where it is now, and the runtime keeps it there as its understanding of the room
    /// changes. Asynchronous, so it is a handle the way a hit-test source is, and a backend with no anchors
    /// says so rather than handing back one that never answers. [`Features::ANCHORS`] says whether they exist.
    fn anchor(&mut self, _space: ReferenceSpace, _pose: Pose) -> Result<Anchor, Error> {
        Err(Error::Unsupported("anchors".into()))
    }

    /// Where an anchor is now, in the space given - `None` when the runtime has lost it, which is a place that
    /// is no longer there rather than a failure. Ask once per frame, like a view.
    fn anchor_pose(
        &mut self,
        _anchor: Anchor,
        _space: ReferenceSpace,
    ) -> Result<Option<Pose>, Error> {
        Ok(None)
    }

    /// Tell the runtime the app is done with an anchor, which is WebXR's `XRAnchor.delete`. Nothing by
    /// default, because a backend with no anchors has none to release.
    fn release_anchor(&mut self, _anchor: Anchor) {}

    /// The runtime's own name for an anchor - WebXR's `XRAnchor.requestPersistentHandle`, OpenXR's
    /// `XrSpatialAnchorCreateCompleteFB`, ARKit's `ar_anchor_get_identifier`.
    /// [`Anchor`]'s id is a slot in *this* session's table and means nothing to the next session, while this
    /// one is the runtime's: it is what a caller writes down to ask for the same place in a later run.
    /// `None` while the runtime has not answered - WebXR answers with a promise - for an anchor this session
    /// did not hand out, and for a runtime with no way to name one.
    fn anchor_id(&mut self, _anchor: Anchor) -> Option<AnchorId> {
        None
    }

    /// Fill in the names the runtime keeps from earlier runs - WebXR's `persistentAnchors`, emptied first.
    /// Empty until it answers or when it keeps none; [`Error::Unsupported`] when it cannot list them.
    fn persistent_anchor_ids(&mut self, out: &mut Vec<AnchorId>) -> Result<(), Error> {
        out.clear();
        Err(Error::Unsupported("anchors from an earlier session".into()))
    }

    /// Ask the runtime where a place it named has ended up, which is WebXR's `restorePersistentAnchor`: the
    /// other half of [`Session::anchor_id`], asynchronous like every anchor, so `anchor_pose` is `None` until
    /// the place is found and stays `None` for a name the runtime does not have. No names kept: `Unsupported`.
    fn restore_anchor(&mut self, _id: AnchorId) -> Result<Anchor, Error> {
        Err(Error::Unsupported("anchors from an earlier session".into()))
    }

    /// Tell the runtime to forget a name for good, which is WebXR's `deletePersistentAnchor`. The counterpart
    /// of [`Session::release_anchor`], which gives back the handle and leaves the name; `Ok` means the request
    /// was made, and a name the runtime does not have is a no-op rather than an error to report.
    fn delete_persistent_anchor(&mut self, _id: AnchorId) -> Result<(), Error> {
        Err(Error::Unsupported("erasing an anchor's name".into()))
    }

    /// What the runtime measured of the real world for the view at `index`, and the buffer itself - WebXR's
    /// `XRFrame.getDepthInformation`, OpenXR's environment depth. `None` for a runtime with no depth sensing,
    /// for a view it has none for, for one that has paused it, and for one that never had any - the same
    /// answer, and why [`Features::DEPTH`] says whether this exists.
    /// Ask once per frame, like a view: depth is a buffer made for the frame and taken back at its end, and the
    /// renderer's importer turns it into something to draw with.
    fn environment_depth(&mut self, _view: usize) -> Option<(&Self::EnvironmentDepth, DepthInfo)> {
        None
    }

    /// Hand the frame back for the compositor to present.
    fn end(&mut self, frame: &mut Frame) -> Result<(), Error>;

    /// Ask the runtime to end this session, which is WebXR's `XRSession.end` and OpenXR's
    /// `xrRequestExitSession` - the app saying it is finished rather than the handle being dropped. Not
    /// [`end`](Session::end), which hands back one frame: this is what a system menu does, and the runtime
    /// answers with a [`StateChanged`](Event::StateChanged) on the next `poll`. A backend with no request to
    /// make keeps the runtime's lifecycle and says so.
    fn request_exit(&mut self) -> Result<(), Error> {
        Err(Error::Unsupported("ending the session".into()))
    }
}
