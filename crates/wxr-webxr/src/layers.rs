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

use wasm_bindgen::prelude::*;

use crate::sys::XrCompositionLayer;

/// A layer the browser is holding.
pub(crate) struct Slot {
    /// The browser's own object. Kept because the render state has to be handed it whenever the list changes,
    /// and because moving a layer is a setter on it rather than a new layer. It is the base class rather than a
    /// shape's, because a session can hold a quad, a cylinder, an equirect and a cube at once - and what tells
    /// them apart for the setters that belong to one shape is `shape`.
    pub(crate) layer: XrCompositionLayer,
    /// What kind of layer it is, and the geometry it was made with - which is how the one setter the core's
    /// `set_layer_pose` needs to reach is found, and how a cube's six faces are said.
    pub(crate) shape: wxr::LayerShape,
    /// Where in its space it is, as the app last placed it. Kept because the core's handle deliberately has no
    /// shape or pose - a caller that asked remembers - and this is what makes the two agree.
    pub(crate) pose: wxr::Pose,
    /// The `redraw` handler, kept because a closure the page no longer holds is one it stops calling. The
    /// handler is per layer and not per session, because the event is the layer's and names which one.
    pub(crate) on_redraw: Option<Closure<dyn FnMut(crate::sys::Event)>>,
}

impl Slot {
    /// Point the browser's `redraw` handler at `handler`.
    ///
    /// The attribute is the shape's rather than the base class's, so this is the one place that has to say
    /// which kind of layer it is holding - the same match `set_layer_pose` makes, for the same reason.
    pub(crate) fn listen_for_redraw(&self, handler: &Closure<dyn FnMut(crate::sys::Event)>) {
        let callback = Some(handler.as_ref().unchecked_ref());
        match self.shape {
            wxr::LayerShape::Quad { .. } => {
                let layer: &crate::sys::XrQuadLayer = self.layer.unchecked_ref();
                layer.set_onredraw(callback);
            }
            wxr::LayerShape::Cylinder { .. } => {
                let layer: &crate::sys::XrCylinderLayer = self.layer.unchecked_ref();
                layer.set_onredraw(callback);
            }
            wxr::LayerShape::Equirect { .. } => {
                let layer: &crate::sys::XrEquirectLayer = self.layer.unchecked_ref();
                layer.set_onredraw(callback);
            }
            wxr::LayerShape::Cube => {
                let layer: &crate::sys::XrCubeLayer = self.layer.unchecked_ref();
                layer.set_onredraw(callback);
            }
        }
    }
}
