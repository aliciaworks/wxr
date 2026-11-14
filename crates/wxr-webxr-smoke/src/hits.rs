//! A hit test along a ray the app named, which is the one call whose numbers a frame cannot supply.
//!
//! Kept out of `tick.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;

/// The ray asked with, and where one metre along it lands.
///
/// The direction is two units long, because a browser's `XRRay` normalises it: a hit at twice the distance
/// would be a ray that was never normalised, and the stub's own `XRRay` is the only place that could have
/// done it. The origin is a different number on each axis, so a pose read out of the wrong order fails.
fn asked() -> wxr::Ray {
    wxr::Ray {
        origin: wxr::glam::Vec3::new(0.25, -0.5, 0.75),
        // Along the space's own `-Z`, which is the direction a source with no ray of its own would use - so
        // a backend that ignored the ray would still land somewhere plausible and be caught by the origin.
        direction: wxr::glam::Vec3::new(0.0, 0.0, -2.0),
    }
}

/// The stub's record of the ray it was handed, as an origin and a direction.
///
/// Read off the global rather than from a call, because the ray is the answer to the *options* the browser
/// was given and no frame can read them back: a source that was asked for along the wrong ray answers a
/// perfectly good hit at the wrong place.
fn recorded() -> Option<(wxr::glam::Vec3, wxr::glam::Vec3)> {
    let global = js_sys::global();
    let value = js_sys::Reflect::get(&global, &JsValue::from_str("__wxrHitTestRay")).ok()?;
    let origin = js_sys::Reflect::get(&value, &JsValue::from_str("origin")).ok()?;
    let direction = js_sys::Reflect::get(&value, &JsValue::from_str("direction")).ok()?;
    Some((point(&origin)?, point(&direction)?))
}

/// Three coordinates off a JavaScript object, or `None` when one of them is not a number.
fn point(value: &JsValue) -> Option<wxr::glam::Vec3> {
    let read = |name: &str| {
        js_sys::Reflect::get(value, &JsValue::from_str(name))
            .ok()
            .and_then(|number| number.as_f64())
    };
    Some(wxr::glam::Vec3::new(
        read("x")? as f32,
        read("y")? as f32,
        read("z")? as f32,
    ))
}

/// Ask once, then read every frame until the source answers, then check where it landed and let it go.
pub(super) fn step(
    board: &mut Board,
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    if board.hit_source.is_none() {
        // The ray is the app's, which is the half of a hit test a space cannot describe - and the half this
        // backend has to put on the options rather than cast itself.
        board.hit_source = Some(
            session
                .hit_test_source(space, Some(asked()))
                .map_err(|error| format!("a hit-test ray of the app's was refused: {error}"))?,
        );
        say("asked for a hit-test source along a ray of the app's own");
    }
    if board.hit_checked {
        return Ok(());
    }
    let source = board.hit_source.expect("made above or in an earlier frame");
    let mut hits = Vec::new();
    session
        .hits(source, space, &mut hits)
        .map_err(|error| format!("hits failed: {error}"))?;
    // The answer arrives a frame or two after the ask, which is the shape the core promises: an empty list
    // is a source whose promise has not landed, and not a ray that missed.
    if hits.is_empty() {
        return Ok(());
    }
    board.hit_checked = true;
    let (origin, direction) =
        recorded().ok_or("the stub recorded no ray for the hit-test source")?;
    let ray = asked();
    if (origin - ray.origin).length() > 1e-6 {
        return Err(format!(
            "the ray's origin arrived as {origin:?}, not {:?}",
            ray.origin
        ));
    }
    let unit = ray.direction.normalize();
    if (direction - unit).length() > 1e-6 {
        return Err(format!(
            "the ray's direction arrived as {direction:?}, not the unit {unit:?}"
        ));
    }
    // One metre along the unit direction, in the space the ray was given in. The arithmetic is the stub's,
    // so what this says is that the ray reached it and that `hits` reads the result back.
    let wanted = ray.origin + unit;
    let landed = hits[0].pose.position;
    if (landed - wanted).length() > 1e-4 {
        return Err(format!("the hit landed at {landed:?}, not {wanted:?}"));
    }
    say("the app's ray reached the browser, and the hit landed one metre along it");
    // And a source let go reports nothing, which is this backend's answer rather than the browser's.
    session.release_hit_test_source(source);
    let mut after = Vec::new();
    session
        .hits(source, space, &mut after)
        .map_err(|error| format!("hits on a released source answered {error}"))?;
    if !after.is_empty() {
        return Err("a released hit-test source still reported a hit".into());
    }
    say("a released hit-test source reports no hits");
    Ok(())
}
