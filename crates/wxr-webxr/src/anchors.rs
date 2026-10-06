//! Anchors: places the runtime keeps fixed in the room.
//!
//! Another module `web-sys` generates nothing for. Its *objects* are declared here, because an `XRAnchor` is a
//! thing this crate passes around; its *entry points* are reached by name, because they hang off `XRFrame`
//! (`createAnchor`, `trackedAnchors`) and `wasm-bindgen` cannot add a method to a type another crate owns. The
//! names come from the [Anchors specification](https://immersive-web.github.io/anchors/).

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrFrame, XrReferenceSpace, XrRigidTransform, XrSpace};

#[wasm_bindgen]
extern "C" {
    /// A place the runtime is keeping fixed relative to the world.
    #[wasm_bindgen(js_name = "XRAnchor")]
    #[derive(Clone)]
    pub type XrAnchor;

    /// The space at the anchor, which is what a pose is asked for. It throws once the anchor is deleted, which
    /// is a place that is gone rather than a failure - so it is caught.
    #[wasm_bindgen(method, getter, catch, js_name = "anchorSpace")]
    pub fn anchor_space(this: &XrAnchor) -> Result<XrSpace, JsValue>;

    /// Tell the runtime the app is done with it, so it can stop tracking the place.
    #[wasm_bindgen(method)]
    pub fn delete(this: &XrAnchor);
}

/// An anchor that has been asked for, and the runtime's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    anchor: Option<XrAnchor>,
}

impl Slot {
    /// The anchor, once the runtime has made one.
    pub fn anchor(&self) -> Option<XrAnchor> {
        self.anchor.clone()
    }

    /// Forget it, which is what asking the runtime to delete one leaves behind.
    pub fn forget(&mut self) {
        self.anchor = None;
    }
}

/// A method by name, which is the only way to reach a member of a type another crate owns.
fn method(target: &JsValue, name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into()
        .ok()
}

/// Ask the frame for an anchor at `pose` in `space`, and fill the slot when the promise answers.
pub fn create(
    frame: &XrFrame,
    space: &XrReferenceSpace,
    pose: XrRigidTransform,
    slot: Rc<RefCell<Slot>>,
) {
    let Some(create) = method(frame.unchecked_ref::<JsValue>(), "createAnchor") else {
        return;
    };
    let Ok(value) = create.call2(
        frame.unchecked_ref::<JsValue>(),
        pose.unchecked_ref::<JsValue>(),
        space.unchecked_ref::<JsValue>(),
    ) else {
        return;
    };
    let Ok(promise) = value.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(anchor) = JsFuture::from(promise).await
            && let Ok(anchor) = anchor.dyn_into::<XrAnchor>()
        {
            slot.borrow_mut().anchor = Some(anchor);
        }
    });
}

/// Where the anchor is now, in `space`, or `None` when the runtime has lost it.
pub fn pose(frame: &XrFrame, anchor: &XrAnchor, base: &XrReferenceSpace) -> Option<wxr::Pose> {
    let space = anchor.anchor_space().ok()?;
    let pose = frame.get_pose(&space, base.unchecked_ref::<XrSpace>())?;
    Some(crate::transform(pose.transform()))
}
