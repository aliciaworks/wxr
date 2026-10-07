//! The pictures a session presents beside the world.
//!
//! A layer is not a view of anything: the compositor takes its texture, puts it where the app said, and warps it
//! for the optics. So the app draws it once, and this crate's whole job is to hold the browser's object and hand
//! out the picture it says to draw into.
//!
//! A layer is made *in* a space and every `transform` given to it afterwards is relative to that space - which is
//! why the core's `Session::set_layer_pose` takes a pose and no space: the layer is where it was made, and a pose
//! handed over in some other space would be a number with a different meaning rather than an error. That is the
//! layer's own business and not this slot's, which is why nothing here remembers the space it was made in.

use crate::sys::XrQuadLayer;

/// A layer the browser is holding.
pub(crate) struct Slot {
    /// The browser's own object. Kept because the render state has to be handed it whenever the list changes,
    /// and because moving a layer is a setter on it rather than a new layer.
    pub(crate) layer: XrQuadLayer,
    /// Where in its space it is, as the app last placed it. Kept because the core's handle deliberately has no
    /// shape or pose - a caller that asked remembers - and this is what makes the two agree.
    pub(crate) pose: wxr::Pose,
}
