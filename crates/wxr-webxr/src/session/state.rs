use super::*;

#[derive(Default)]
pub(super) struct Inner {
    pub(super) session: Option<XrSession>,
    /// The frame the animation callback put here, and the time it was for, until `begin` takes it.
    pub(super) frame: Option<(XrFrame, Duration)>,
    /// Whether a frame has been asked for and has not arrived.
    ///
    /// It is what keeps `request_frame` from asking twice: the browser holds one callback at a time, and the
    /// second ask would replace the first - so the callback it then calls is a closure this side has dropped,
    /// which is an exception in the animation callback rather than a frame.
    pub(super) awaiting: bool,
}

/// A reference space, which the browser hands over as a promise rather than as an object.
///
/// The promise is kept while it is pending because an offset space is made *from* another one and has to wait
/// for the same answer - which is the only reason this is not just an `Option`.
#[derive(Default)]
pub(super) struct Space {
    pub(super) space: Option<XrReferenceSpace>,
    pub(super) promise: Option<js_sys::Promise>,
    /// The `reset` handler, kept alive for as long as the space is: a closure the browser holds and this drops
    /// is a closure that stops being called. Nothing reads it - holding it is the work.
    #[allow(dead_code)]
    pub(super) on_reset: Option<Closure<dyn FnMut(Event)>>,
}

impl Space {
    /// A space that is here.
    pub(super) fn resolved(space: XrReferenceSpace) -> Self {
        Self {
            space: Some(space),
            promise: None,
            on_reset: None,
        }
    }
}

/// One of the frame's images: the colour texture, and the depth buffer the layer gave with it.
///
/// They are one thing here because the compositor hands them over as one - both come from the same sub-image -
/// and because that is what lets an importer answer for both. A layer made without a depth format has no depth
/// texture, which is why this one is an `Option`: the specification says it is nullable.
pub struct FrameImage {
    pub color: JsValue,
    pub depth: Option<JsValue>,
}

/// The binding and the layer, which only exist together: a binding with no layer presents nothing.
pub(super) struct Gpu {
    pub(super) binding: XrgpuBinding,
    pub(super) layer: XrProjectionLayer,
}

/// A live WebXR session.
pub struct WebXrSession {
    pub(super) inner: Rc<RefCell<Inner>>,
    pub(super) connect: Rc<RefCell<Connect>>,
    /// The animation callback, kept alive for as long as the session is: a closure that is dropped is a
    /// closure the browser stops calling.
    pub(super) callback: Option<Closure<dyn FnMut(f64, XrFrame)>>,
    /// The frame `begin` took out of the slot, kept for the one call that needs it: `views` asks it for the
    /// viewer pose. `begin` used to take it and drop it, and `views` looked in the slot it was taken from -
    /// which is a session that locates no views, ever, and so draws nothing.
    pub(super) current: Option<XrFrame>,
    /// The `end` handler's flag, and the handler itself - same reason as the callback: a closure that is
    /// dropped is a closure the browser stops calling.
    pub(super) ended: Rc<Cell<bool>>,
    pub(super) on_end: Option<Closure<dyn FnMut()>>,
    /// Whether `Lost` has been said, so that the end of a session is news once.
    pub(super) lost: bool,
    /// Spaces, each a slot: the request is a promise, so a space exists a frame or two after it is asked for.
    pub(super) spaces: Vec<Rc<RefCell<Space>>>,
    pub(super) state: wxr::State,
    /// Whether the session is being shown, which is the browser's own `visibilityState`.
    pub(super) visibility: wxr::Visibility,
    /// The sources this session has seen, so an id from a frame and an id from an event are the same answer.
    pub(super) sources: input::Sources,
    /// The subscription to the session's six input events, once there is a session to subscribe to.
    pub(super) input: Option<input::Events>,
    /// The surfaces this session has seen, for the same reason as the sources: WebXR names one by the object it
    /// is, and a core plane carries a number.
    pub(super) planes: planes::Ids,
    /// The hit-test sources this session has asked for, each empty until the browser answers.
    pub(super) hit_sources: Vec<Rc<RefCell<hit::Slot>>>,
    /// The light probes this session has asked for, each empty until the browser answers.
    pub(super) light_probes: Vec<Rc<RefCell<light::Slot>>>,
    /// This frame's depth buffer, kept for as long as the reference into it is handed out.
    pub(super) depth_image: Option<JsValue>,
    /// How much foveation the app asked for, if it asked: `None` leaves the layer's own default alone.
    pub(super) foveation: Option<f32>,
    /// The anchors this session has asked for, each empty until the browser answers.
    pub(super) anchors: Vec<Rc<RefCell<anchors::Slot>>>,
    /// This frame's views, kept because depth is asked for one of them by object and not by index.
    pub(super) frame_views: Vec<XrView>,
    /// Reference-space `reset` events, which arrive on a space rather than on the session and are passed on
    /// from here.
    pub(super) reset: Rc<RefCell<VecDeque<wxr::Event>>>,
    /// The views the frame located, for the layer the compositor would be given.
    pub(super) located: usize,
    /// The device the app made, kept because a WebXR/WebGPU session needs it: the binding that hands out the
    /// textures is built from it, once, when the session arrives.
    pub(super) device: Option<wgpu::Device>,
    /// The binding and the layer, when the browser gave both. `None` is a session with a head, two eyes and a
    /// clock and no picture - which is what this backend has always been able to be, and says so.
    pub(super) gpu: Option<Gpu>,
    /// This frame's image, which both eyes share: one colour texture, and the depth that came with it.
    pub(super) image: Option<FrameImage>,
    /// What that texture said about itself, so `images` does not have to ask it twice.
    pub(super) meta: wxr::ImageMeta,
    /// The near and far planes the app asked for, kept because a session that has not arrived has no render
    /// state to put them on yet.
    pub(super) depth_range: Option<(f32, f32)>,
    /// The layers the app has made, each `None` once released. The index *is* the handle's id, which is why a
    /// release leaves a hole rather than shifting the ones after it.
    pub(super) layers: Vec<Option<layers::Slot>>,
    /// This frame's picture of each layer, kept for as long as the reference into it is handed out - the same
    /// reason `depth_image` is kept, and the same reason it is dropped at the start of a frame.
    pub(super) layer_images: Vec<Option<FrameImage>>,
}
