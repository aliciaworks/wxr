//! The anchors, which need a frame to make one in and the frames after it to check.
//!
//! Kept out of `tick.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;

/// The place the anchor is asked for, relative to the floor.
///
/// A different number on each axis, so a pose read back out of the wrong order is a failed check rather than a
/// passing one.
fn wanted() -> wxr::Pose {
    wxr::Pose {
        position: wxr::glam::Vec3::new(1.0, 2.0, 3.0),
        ..wxr::Pose::IDENTITY
    }
}

/// How many times the stub was asked for an anchor's persistent handle.
fn handle_requests() -> u32 {
    js_sys::Reflect::get(
        &js_sys::global(),
        &JsValue::from_str("__wxrPersistentHandleRequests"),
    )
    .ok()
    .and_then(|value| value.as_f64())
    .map(|count| count as u32)
    .unwrap_or(0)
}

/// How many anchors the stub has been told to delete.
fn deleted() -> u32 {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrAnchorsDeleted"))
        .ok()
        .and_then(|value| value.as_f64())
        .map(|count| count as u32)
        .unwrap_or(0)
}

/// Ask for an anchor with no frame open, which is the refusal that is about the frame rather than the space.
///
/// `createAnchor` needs an active frame, so the answer is `Unavailable` and not a browser throw - and the ask
/// is made before the floor's frame so that the one thing being shown is the missing frame.
pub(super) fn unframed(
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    match session.anchor(space, wanted()) {
        Err(wxr::Error::Unavailable(_)) => Ok(()),
        Err(error) => Err(format!(
            "an anchor asked for outside a frame was refused as {error}, where the missing frame is the answer"
        )),
        Ok(_) => Err("an anchor was made outside a frame".into()),
    }
}

/// One step of the anchor checks, called once per tick while a frame is open.
///
/// The steps are the browser's order as much as this backend's: an anchor arrives as a promise, so the tick
/// that asks has no anchor yet, the next one has an anchor but maybe no name, and the one after that has
/// both. Every step is checked against what the *stub* recorded as well as against what the backend answers,
/// because a backend that never called the browser would answer `None` at every step and look consistent.
pub(super) fn step(
    board: &mut Board,
    session: &mut wxr_webxr::WebXrSession,
    space: wxr::ReferenceSpace,
) -> Result<(), String> {
    match board.anchor_step {
        0 => {
            let anchor = session
                .anchor(space, wanted())
                .map_err(|error| format!("an anchor in the floor space: {error}"))?;
            board.anchor = Some(anchor);
            board.anchor_step = 1;
            say("asked for an anchor at (1, 2, 3) in the floor space");
        }
        1 => {
            let anchor = board.anchor.expect("step one is after an anchor was made");
            let pose = session
                .anchor_pose(anchor, space)
                .map_err(|error| format!("the anchor's pose: {error}"))?
                .ok_or("an anchor made last frame has no pose in this one")?;
            let want = wanted();
            if (pose.position - want.position).length() > 1e-3 {
                return Err(format!(
                    "the anchor came back at ({:.3}, {:.3}, {:.3}), where it was asked for at ({:.3}, {:.3}, {:.3})",
                    pose.position.x,
                    pose.position.y,
                    pose.position.z,
                    want.position.x,
                    want.position.y,
                    want.position.z,
                ));
            }
            // Asked while the browser's answer may still be a promise, which is what makes the next step's
            // answer the one that came back rather than the one the ask returned.
            let _ = session.anchor_id(anchor);
            board.anchor_step = 2;
            say("the anchor is where it was asked for");
        }
        2 => {
            let anchor = board.anchor.expect("step two is after an anchor was made");
            // A promise the browser has not read yet is not a missing name, so a step that has no answer yet
            // is one to try again on the next frame rather than one to fail.
            let Some(id) = session.anchor_id(anchor) else {
                return Ok(());
            };
            if handle_requests() != 1 {
                return Err(format!(
                    "the browser was asked for a persistent handle {} time(s), where one anchor has one name",
                    handle_requests()
                ));
            }
            // Written down and read back, which is the whole use of the name: a caller saves it and asks the
            // next session for the same place. Here the round trip is only proof that the bytes survived.
            let text = id.to_string();
            if text.parse::<wxr::AnchorId>() != Ok(id) {
                return Err(format!(
                    "the anchor's name {id} did not survive its own text"
                ));
            }
            session.release_anchor(anchor);
            board.anchor_step = 3;
            say(&format!("the browser named the anchor {id}, once"));
        }
        3 => {
            let anchor = board
                .anchor
                .expect("step three is after an anchor was made");
            match session.anchor_pose(anchor, space) {
                Ok(None) => {}
                Ok(Some(_)) => return Err("a released anchor still has a pose".into()),
                Err(error) => return Err(format!("a released anchor's pose answered {error}")),
            }
            if session.anchor_id(anchor).is_some() {
                return Err("a released anchor still has a name".into());
            }
            if deleted() != 1 {
                return Err(format!(
                    "the browser was told to delete {} anchor(s), where one was released",
                    deleted()
                ));
            }
            board.anchor_step = 4;
            say("the released anchor has no pose, no name, and was deleted once");
        }
        _ => {}
    }
    Ok(())
}
