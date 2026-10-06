//! The room's light, which is WebXR's lighting estimation.
//!
//! `light-estimation` is another module `web-sys` generates nothing for, so every call is made by name - the
//! way the plane and hit-test modules are and for the same reason. A probe is created asynchronously and is
//! opaque afterwards, so one is a `JsValue` that waits in a slot.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrFrame, XrSession};

/// A probe that has been asked for, and the browser's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    probe: Option<JsValue>,
}

impl Slot {
    /// The probe, if the browser has answered.
    pub fn probe(&self) -> Option<JsValue> {
        self.probe.clone()
    }
}

/// A method by name, which is the only way to reach a module `web-sys` does not know.
fn method(target: &JsValue, name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into()
        .ok()
}

/// Ask the browser for a probe, and fill the slot when it answers.
///
/// No options: the only one the specification has is the reflection format of a cubemap this core does not read,
/// and the probe's own space is the runtime's to choose either way.
pub fn request(session: &XrSession, slot: Rc<RefCell<Slot>>) {
    let Some(request) = method(session.unchecked_ref::<JsValue>(), "requestLightProbe") else {
        return;
    };
    let Ok(value) = request.call0(session.unchecked_ref::<JsValue>()) else {
        return;
    };
    let Ok(promise) = value.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(probe) = JsFuture::from(promise).await {
            slot.borrow_mut().probe = (!probe.is_null_or_undefined()).then_some(probe);
        }
    });
}

/// Read the frame's estimate for a probe into the core's shape.
pub fn estimate(frame: &XrFrame, probe: &JsValue, out: &mut wxr::LightEstimate) {
    let Some(get) = method(frame.unchecked_ref::<JsValue>(), "getLightEstimate") else {
        return;
    };
    let Ok(estimate) = get.call1(frame.unchecked_ref::<JsValue>(), probe) else {
        return;
    };
    if estimate.is_null_or_undefined() {
        return;
    }

    // `sphericalHarmonicsCoefficients` is 27 floats: nine basis functions, red green blue each - so a
    // coefficient is three in a row, and there are nine of them.
    let mut harmonics = [wxr::glam::Vec3::ZERO; 9];
    let values = js_sys::Reflect::get(
        &estimate,
        &JsValue::from_str("sphericalHarmonicsCoefficients"),
    )
    .ok()
    .and_then(|value| value.dyn_into::<js_sys::Float32Array>().ok());
    if let Some(values) = values {
        // A copy of the 27 floats: a `Float32Array` is read whole, and one small enough that reading by index
        // would be the only thing missing a method for.
        let values = values.to_vec();
        for (index, coefficient) in harmonics.iter_mut().enumerate() {
            let at = index * 3;
            if at + 2 < values.len() {
                *coefficient = wxr::glam::Vec3::new(values[at], values[at + 1], values[at + 2]);
            }
        }
    }

    // The primary light is a pair of points, which is how WebXR carries a vector and a colour.
    let point = |name: &str| {
        let value = js_sys::Reflect::get(&estimate, &JsValue::from_str(name)).ok()?;
        let at = |axis: &str| {
            js_sys::Reflect::get(&value, &JsValue::from_str(axis))
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0) as f32
        };
        Some(wxr::glam::Vec3::new(at("x"), at("y"), at("z")))
    };

    *out = wxr::LightEstimate {
        primary_direction: point("primaryLightDirection").unwrap_or_default(),
        primary_intensity: point("primaryLightIntensity").unwrap_or_default(),
        harmonics,
    };
}
