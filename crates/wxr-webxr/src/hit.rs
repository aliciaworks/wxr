//! Hit testing: where a ray out of a space meets the world.
//!
//! `hit-test` is another module `web-sys` generates nothing for, so every call is made by name - the way the
//! plane module is read and for the same reason. A source is created asynchronously and is otherwise opaque, so
//! the whole of one is a `JsValue` that waits in a slot until the browser answers.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrFrame, XrReferenceSpace, XrSession};

/// A source that has been asked for, and the browser's answer once it arrives.
///
/// The slot is empty until the promise resolves, which is one or two frames - and a frame with no source yet is
/// a frame with no hits, not a frame to fail.
#[derive(Default)]
pub struct Slot {
    source: Option<JsValue>,
}

impl Slot {
    /// The source, if the browser has answered.
    pub fn source(&self) -> Option<JsValue> {
        self.source.clone()
    }
}

/// A method by name, which is the only way to reach a module `web-sys` does not know.
fn method(target: &JsValue, name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into()
        .ok()
}

/// Ask the browser for a source aiming out of `space`, and fill the slot when it answers.
pub fn request(session: &XrSession, space: &XrReferenceSpace, slot: Rc<RefCell<Slot>>) {
    // `XRHitTestOptionsInit` is a plain dictionary, so an object with a `space` on it *is* one - which is how a
    // module with no bindings is asked for anything.
    let options = js_sys::Object::new();
    if js_sys::Reflect::set(
        &options,
        &JsValue::from_str("space"),
        space.unchecked_ref::<JsValue>(),
    )
    .is_err()
    {
        return;
    }
    let Some(request) = method(session.unchecked_ref::<JsValue>(), "requestHitTestSource") else {
        return;
    };
    let Ok(value) = request.call1(session.unchecked_ref::<JsValue>(), &options) else {
        return;
    };
    let Ok(promise) = value.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(source) = JsFuture::from(promise).await {
            let mut slot = slot.borrow_mut();
            slot.source = (!source.is_null_or_undefined()).then_some(source);
        }
    });
}

/// Read this frame's hits along `source`, expressed in `base`, into the core's shape.
pub fn results(
    frame: &XrFrame,
    source: &JsValue,
    base: &XrReferenceSpace,
    out: &mut Vec<wxr::Hit>,
) {
    let Some(results) = method(frame.unchecked_ref::<JsValue>(), "getHitTestResults") else {
        return;
    };
    let Ok(value) = results.call1(frame.unchecked_ref::<JsValue>(), source) else {
        return;
    };
    let Ok(Some(results)) = js_sys::try_iter(&value) else {
        return;
    };
    for result in results.flatten() {
        // Each result is a pose in the base space, which is the only thing a hit test says. A result the
        // browser will not answer for is one that missed this frame.
        let Some(get_pose) = method(&result, "getPose") else {
            continue;
        };
        let Ok(pose) = get_pose.call1(&result, base.unchecked_ref::<JsValue>()) else {
            continue;
        };
        if pose.is_null_or_undefined() {
            continue;
        }
        let Some(transform) = method(&pose, "transform").and_then(|call| call.call0(&pose).ok())
        else {
            continue;
        };
        let Ok(transform) = transform.dyn_into::<web_sys::XrRigidTransform>() else {
            continue;
        };
        out.push(wxr::Hit {
            pose: crate::transform(transform),
        });
    }
}
