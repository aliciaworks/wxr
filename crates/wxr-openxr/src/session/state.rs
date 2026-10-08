use super::*;

pub struct OpenXrSession {
    /// Which composition-layer extensions this instance was made with, and so which shapes it will take.
    pub(super) layers_enabled: LayerExtensions,
    /// The instance is kept for its event queue: OpenXR polls events from the instance, not the session.
    pub(super) instance: xr::Instance,
    pub(super) events: xr::EventDataBuffer,
    pub(super) session: xr::Session<xr::Vulkan>,
    /// Waiting and submitting are two handles, and the wait is where the frame's timing comes from.
    pub(super) waiter: xr::FrameWaiter,
    pub(super) stream: xr::FrameStream<xr::Vulkan>,
    pub(super) swapchain: xr::Swapchain<xr::Vulkan>,
    /// The compositor's images, as the runtime names them - a `VkImage`, which on this platform is an
    /// integer. The renderer wraps these; the core carries them.
    pub(super) images: Vec<u64>,
    pub(super) extent: wxr::Extent2d,
    /// What the swapchain's images are. This backend chooses it from what the runtime offers, so it is the one
    /// that has to report it - and the renderer builds its pipeline from the answer.
    pub(super) color: wxr::ColorFormat,
    pub(super) blend: xr::EnvironmentBlendMode,
    pub(super) spaces: Vec<xr::Space>,
    /// Where each of `spaces` sits inside its kind, because an offset space is the kind's origin moved - and an
    /// offset of an offset has to add up.
    pub(super) offsets: Vec<wxr::Pose>,
    /// The viewer's own space, whose one job is the head pose: OpenXR reports where the eyes are and not where
    /// the wearer is, and `VIEW` is the single reference space that is the wearer.
    pub(super) view: xr::Space,
    /// The hands, which are declared once and read once a frame. `None` when the runtime would not take
    /// the action set - a runtime with no controllers is a session with no inputs rather than no session.
    pub(super) hands: Option<input::Hands>,
    /// The views the runtime located for the frame in progress. Kept because the composition layer is
    /// built from them, and they cannot be recovered from the core's view type without a round trip that
    /// would have to be exact to be honest.
    pub(super) located: Vec<xr::View>,
    /// The lifecycle and the visibility the core speaks, derived from OpenXR's own ladder below.
    pub(super) state: wxr::State,
    pub(super) visibility: wxr::Visibility,
    /// OpenXR's own state, kept because it carries more than the core's two axes do - and a program that
    /// needs the rest needs the platform.
    pub(super) openxr_state: xr::SessionState,
    pub(super) predicted: xr::Time,
    /// Which image the frame took, until it is given back.
    pub(super) held: Option<u32>,
    /// Whether this session has been begun, which is a thing OpenXR makes the app do once.
    pub(super) begun: bool,
    /// Whether `Lost` has been said, so the end of a session is news once.
    pub(super) lost: bool,
    /// The foveation profile the app last asked for, kept because the swapchain points at it - dropping it
    /// would leave the swapchain with a dangling one.
    pub(super) foveation: Option<xr::FoveationProfileFB>,
    /// The swapchain format as the runtime names it, kept because a layer's swapchain has to be made in a
    /// format the runtime offered too - and the same one is the honest choice: the renderer's pipeline is
    /// built once.
    pub(super) format: u32,
    /// The layers the app made, each `None` once released. The index is the handle's id, so a release leaves a
    /// hole rather than shifting the ones after it.
    pub(super) layers: Vec<Option<Layer>>,
}

/// A layer the app made, and the swapchain it draws into.
///
/// One swapchain per layer, which is what a composition layer *is* on OpenXR: the runtime reads the image the
/// layer names at `xrEndFrame`, and a swapchain is how an app hands one over without the compositor and the
/// renderer sharing a fence. The projection layer is the same machinery and is not one of these, because a
/// session has it whether an app asked for one or not.
pub(super) struct Layer {
    pub(super) swapchain: xr::Swapchain<xr::Vulkan>,
    /// Its images, as the runtime names them - a `VkImage`, which on this platform is an integer - for the
    /// renderer to wrap, the same shape the eyes' images arrive in.
    pub(super) images: Vec<u64>,
    /// The space it was made in, by the session's own index, and where in it. Both are needed at submission,
    /// because a layer's pose is read then and not when it is placed.
    pub(super) space: usize,
    pub(super) pose: xr::Posef,
    /// The shape, which is what `xrEndFrame` is told about it - and the reason a layer is not just a size:
    /// `XR_KHR_composition_layer_*` is one extension per shape and each has its own struct.
    pub(super) shape: wxr::LayerShape,
    /// And the image's size in pixels, which is the resolution the app asked to draw at - a different fact, and
    /// the one a renderer makes its target from.
    pub(super) extent: wxr::Extent2d,
    /// Which image this frame took, until it is given back.
    pub(super) held: Option<u32>,
}

/// Which composition-layer extensions the instance was made with.
///
/// The quad is in the core specification; the other three are one extension each, and an extension is asked
/// for when the instance is made and not afterwards. So this is a fact about the instance that both the
/// capability bits and the per-shape refusal need, kept in one place rather than asked twice.
#[derive(Clone, Copy, Default)]
pub(crate) struct LayerExtensions {
    pub(crate) cylinder: bool,
    pub(crate) equirect: bool,
    pub(crate) cube: bool,
}
