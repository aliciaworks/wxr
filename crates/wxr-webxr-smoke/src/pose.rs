//! Where one space is seen from another, which is WebXR's `XRFrame.getPose`.

use super::*;

/// Ask where the viewer is from the floor and the floor from the viewer, once the floor has resolved.
///
/// The two answers are one conversation: `locate(viewer, floor)` is the head above the floor and
/// `locate(floor, viewer)` is the floor below the head, so a backend that swapped its arguments would report
/// the same +1.7 m twice. The walk the stub gives the head is what makes the numbers say which way round it
/// went, and the two poses undoing each other is a stronger statement than either of them alone.
pub(super) fn step(board: &mut Board, session: &mut wxr_webxr::WebXrSession) -> Result<(), String> {
    let Some(floor) = board.space else {
        return Ok(());
    };
    let viewer = match board.viewer {
        Some(viewer) => viewer,
        None => {
            let viewer = session
                .space(wxr::SpaceKind::Viewer)
                .map_err(|error| format!("the viewer space: {error}"))?;
            board.viewer = Some(viewer);
            // A space is a promise, so the frame that asked for it has nothing to locate yet.
            return Ok(());
        }
    };
    let Some(up) = session
        .locate(viewer, floor)
        .map_err(|error| format!("the viewer in the floor: {error}"))?
    else {
        return Ok(());
    };
    let down = session
        .locate(floor, viewer)
        .map_err(|error| format!("the floor in the viewer: {error}"))?
        .ok_or("the floor is nowhere in the viewer's space")?;

    // The stub's head is 1.7 m over the floor and swaying a few centimetres, so being over it at all is what
    // separates the two directions - and the pair being inverse is what says neither one was copied.
    if !(1.5..2.0).contains(&up.pose.position.y) {
        return Err(format!(
            "the viewer is at {:.3} m over the floor, where the stub's head is about 1.7",
            up.pose.position.y
        ));
    }
    if down.pose.position.y >= 0.0 {
        return Err(format!(
            "the floor is at {:.3} m over the viewer, where it belongs under it",
            down.pose.position.y
        ));
    }
    let apart = (up.pose.position + down.pose.position).length();
    if apart > 1e-6 {
        return Err(format!(
            "the two answers do not undo each other: {apart:.4} m apart"
        ));
    }
    // The stub reports a speed on every pose, both halves, and says none of these places was measured.
    let speed = up
        .motion
        .ok_or("a pose the stub gave a speed for arrived with no motion")?;
    if speed.linear_velocity.is_none() || speed.angular_velocity.is_none() {
        return Err(format!(
            "the viewer's motion came through as {speed:?}, halves missing"
        ));
    }
    if up.emulated_position {
        return Err("a reference space's pose was called an emulated position".into());
    }
    // And a handle this session never made is refused, which is the half of the answer a caller acts on.
    let forged = wxr::ReferenceSpace::new(wxr::SpaceKind::LocalFloor, 4096);
    if !matches!(
        session.locate(forged, floor),
        Err(wxr::Error::NoSpace(wxr::SpaceKind::LocalFloor))
    ) {
        return Err("a forged space was located rather than refused".into());
    }

    say(&format!(
        "the viewer is {:.2} m over the floor, the floor {:.2} m under it, and a forged space was NoSpace",
        up.pose.position.y, -down.pose.position.y
    ));
    board.pose_checked = true;
    Ok(())
}
