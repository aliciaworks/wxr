//! The room's light, which is WebXR's lighting estimation.
//!
//! A probe is created asynchronously and is opaque afterwards, so one waits in a slot until the browser
//! answers; what it is *for* is asked of the frame, which is where an estimate is.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::sys::{XrFrame, XrLightProbe, XrSession};

/// A probe that has been asked for, and the browser's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    probe: Option<XrLightProbe>,
}

impl Slot {
    /// The probe, if the browser has answered.
    pub fn probe(&self) -> Option<XrLightProbe> {
        self.probe.clone()
    }
}

/// Ask the browser for a probe, and fill the slot when it answers.
///
/// No options: the only one the specification has is the reflection format of a cubemap this core does not read,
/// and the probe's own space is the runtime's to choose either way.
pub fn request(session: &XrSession, slot: Rc<RefCell<Slot>>) {
    let promise: js_sys::Promise = session.request_light_probe();
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(probe) = JsFuture::from(promise).await
            && let Ok(probe) = probe.dyn_into::<XrLightProbe>()
        {
            slot.borrow_mut().probe = Some(probe);
        }
    });
}

/// Read the frame's estimate for a probe into the core's shape.
pub fn estimate(frame: &XrFrame, probe: &XrLightProbe, out: &mut wxr::LightEstimate) {
    let Some(estimate) = frame.get_light_estimate(probe) else {
        // A probe the runtime has nothing to say about this frame is not a failure: a room's light does not
        // change between frames, and the last estimate is still true.
        return;
    };

    // `sphericalHarmonicsCoefficients` is 27 floats: nine basis functions, red green blue each - so a
    // coefficient is three in a row, and there are nine of them.
    let mut harmonics = [wxr::glam::Vec3::ZERO; 9];
    let values = estimate.spherical_harmonics_coefficients();
    for (index, coefficient) in harmonics.iter_mut().enumerate() {
        let at = index * 3;
        if at + 2 < values.len() {
            *coefficient = wxr::glam::Vec3::new(values[at], values[at + 1], values[at + 2]);
        }
    }

    // The primary light is a pair of points, which is how WebXR carries a vector and a colour: the direction is
    // a point at that distance, the intensity a point with a channel per axis.
    let point = |value: crate::sys::DomPointReadOnly| {
        wxr::glam::Vec3::new(value.x() as f32, value.y() as f32, value.z() as f32)
    };

    *out = wxr::LightEstimate {
        primary_direction: point(estimate.primary_light_direction()),
        primary_intensity: point(estimate.primary_light_intensity()),
        harmonics,
    };
}
