//! A hit test along a ray the app named, and one that follows the controllers' own aim.
//!
//! Kept out of `tick.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;

/// The profile the stub's two controllers carry, which is how a hit test names who aims.
const PROFILE: &str = "wxr-stub";

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

/// Whether the stub has answered a transient hit test at all, which tells an empty answer from no answer:
/// the promise has landed when it has been asked, and only then does an empty list mean no source matched.
fn answered() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrTransientCalls"))
        .ok()
        .and_then(|value| value.as_f64())
        .is_some_and(|calls| calls > 0.0)
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

/// Ask once, read every frame until the source answers, check where it landed and let it go - then do the
/// same for the two controllers' own aim, which is one hit each and one id each.
pub(super) fn step(
    board: &mut Board,
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    match board.hit_step {
        0 | 1 => ray(board, session, space),
        2 | 3 => profile(board, session, space),
        _ => Ok(()),
    }
}

/// The ray the app named: the half of a hit test a space cannot describe.
fn ray(
    board: &mut Board,
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    if board.hit_step == 0 {
        // The ray is the app's, which is the half of a hit test a space cannot describe - and the half this
        // backend has to put on the options rather than cast itself.
        board.hit_source = Some(
            session
                .hit_test_source(space, wxr::HitTestKinds::ALL, Some(asked()))
                .map_err(|error| format!("a hit-test ray of the app's was refused: {error}"))?,
        );
        board.hit_step = 1;
        say("asked for a hit-test source along a ray of the app's own");
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
    if hits[0].input.is_some() {
        return Err("a ray the app named came back as one of its pointers'".into());
    }
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
    board.hit_step = 2;
    Ok(())
}

/// The same question asked by profile, which is answered once per controller that carries it.
fn profile(
    board: &mut Board,
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    if board.hit_step == 2 {
        board.hit_source = Some(
            session
                .hit_test_source_for_profile(wxr::HitTestKinds::PLANES, PROFILE)
                .map_err(|error| format!("a hit-test source for {PROFILE} was refused: {error}"))?,
        );
        board.hit_step = 3;
        say("asked for a hit-test source that follows the controllers' profile");
    }
    let source = board.hit_source.expect("made above or in an earlier frame");
    let mut hits = Vec::new();
    session
        .hits(source, space, &mut hits)
        .map_err(|error| format!("hits on a profile source failed: {error}"))?;
    if hits.is_empty() {
        // The promise has landed but nothing matched, which no later frame is going to change - a profile is
        // asked for because something carries it. Before that, the ask is simply still in flight.
        if answered() {
            return Err("a profile both controllers carry answered no hits".into());
        }
        return Ok(());
    }
    // The ids a hit names have to be the ones `inputs` names, or the app has a pointer it cannot find: the
    // two tables are the same table, and this is where that is checked.
    let mut sources = Vec::new();
    session
        .inputs(space, &mut sources)
        .map_err(|error| format!("inputs failed: {error}"))?;
    let mut matching = Vec::new();
    for input in sources {
        let mut profiles = Vec::new();
        session
            .profiles(input.id, &mut profiles)
            .map_err(|error| format!("profiles failed: {error}"))?;
        if profiles.iter().any(|name| name == PROFILE) {
            matching.push(input);
        }
    }
    if hits.len() != matching.len() {
        return Err(format!(
            "a profile source answered {} hits for {} sources carrying {PROFILE}",
            hits.len(),
            matching.len()
        ));
    }
    for input in &matching {
        let hit = hits
            .iter()
            .find(|hit| hit.input == Some(input.id))
            .ok_or_else(|| format!("no hit named {:?}, which carries {PROFILE}", input.id))?;
        // One metre along the source's own `-Z`, which is the stub's arithmetic done from the pose the app
        // read: the same number by two routes.
        let forward = input.aim.orientation * wxr::glam::Vec3::NEG_Z;
        let wanted = input.aim.position + forward;
        if (hit.pose.position - wanted).length() > 1e-4 {
            return Err(format!(
                "the hit for {:?} landed at {:?}, not {wanted:?}",
                input.id, hit.pose.position
            ));
        }
    }
    say("the profile source answered once per controller, each hit naming the one that found it");
    // The other kind of source is released the same way, and its browser object's `cancel` is the one the
    // ray source's is not.
    session.release_hit_test_source(source);
    let mut after = Vec::new();
    session
        .hits(source, space, &mut after)
        .map_err(|error| format!("hits on a released profile source answered {error}"))?;
    if !after.is_empty() {
        return Err("a released profile source still reported a hit".into());
    }
    say("a released profile source reports no hits");
    board.hit_step = 4;
    Ok(())
}
