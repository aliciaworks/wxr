//! One frame of the smoke loop.
//!
//! Kept out of `lib.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;

/// One tick of the loop: what happened, where the eyes are, and a frame handed back.
///
/// `Err` is a run that cannot get to the next frame, with the sentence a person would want to read.
pub(super) fn tick(board: &mut Board, session: &mut wxr_webxr::WebXrSession) -> Result<(), String> {
    board.ticks += 1;
    while let Some(event) = session.poll() {
        say(&format!("event: {event:?}"));
    }
    // Reported on every change rather than per frame: a session that never reaches `Visible` is exactly the
    // case this backend has to be honest about, and frames never arrive to say it in. The lifecycle and the
    // visibility are separate axes, so each is reported when it moves.
    if session.state() != board.state || session.visibility() != board.visibility {
        board.state = session.state();
        board.visibility = session.visibility();
        let meta = session.images();
        say(&format!(
            "state {:?}, visibility {:?}: {} image(s), {:?} at {:?}",
            board.state,
            board.visibility,
            session.image_count(),
            meta.format,
            meta.extent
        ));
        // Asked on every rung rather than once at the end: the features are the session's and a session that
        // has not arrived has none, so this is the rung that answers with something.
        say(&format!("features: {:?}", session.features()));
    }

    if board.space.is_none() && session.visibility().can_render() {
        match session.space(wxr::SpaceKind::LocalFloor) {
            Ok(space) => {
                say("asked for a floor space");
                board.space = Some(space);
            }
            Err(error) => {
                return Err(format!("no floor space: {error}"));
            }
        }
    }
    let Some(space) = board.space else {
        return Ok(());
    };

    let now = Duration::from_millis(u64::from(board.ticks) * 16);
    if let Err(error) = session.begin(now, &mut board.frame) {
        return Err(format!("begin failed: {error}"));
    }
    if !board.frame.is_render() {
        return Ok(());
    }
    if let Err(error) = session.views(space, &mut board.frame) {
        say(&format!("views failed: {error}"));
    }
    for view in board.frame.views() {
        say(&format!(
            "  {:?} eye at ({:.3}, {:.3}, {:.3}), fov {:.3}/{:.3}/{:.3}/{:.3}, viewport {}x{}+{}+{}, layer {} of image {}",
            view.eye,
            view.pose.position.x,
            view.pose.position.y,
            view.pose.position.z,
            view.fov.up,
            view.fov.down,
            view.fov.left,
            view.fov.right,
            view.viewport.width,
            view.viewport.height,
            view.viewport.x,
            view.viewport.y,
            view.layer,
            view.image,
        ));
    }
    // The image path, asserted once rather than per frame. Only once there are eyes: the floor space is a
    // promise, and the frame that asked for it has none yet.
    if !board.checked && board.frame.is_render() && !board.frame.views().is_empty() {
        board.checked = true;
        images::check_images(board, session)?;
    }
    // The sources, which are the frame's half of input: where the hands are and how they are aimed. What they
    // *do* arrives as events above, because that is where WebXR puts it.
    let mut sources = Vec::new();
    if let Err(error) = session.inputs(space, &mut sources) {
        say(&format!("inputs failed: {error}"));
    }
    if !board.input_said && !sources.is_empty() {
        board.input_said = true;
        for source in &sources {
            say(&format!(
                "  {:?} hand, {:?}, id {}, aiming from ({:.3}, {:.3}, {:.3})",
                source.handedness,
                source.target_ray_mode,
                source.id.get(),
                source.aim.position.x,
                source.aim.position.y,
                source.aim.position.z,
            ));
        }
    }
    // The skeletons, and one buzz each, both asked once. A joint is asked of a *hand* and a hand belongs to a
    // source, so the sources above are what a hand is found by - and a run that asked every frame would be a
    // run whose record is the same line twenty times over.
    if !board.hands_said && !sources.is_empty() {
        board.hands_said = true;
        for source in &sources {
            let index = source.id.get() as usize;
            while board.hands.len() <= index {
                board.hands.push(wxr::Hand::default());
            }
            if let Err(error) = session.hand(source.id, space, &mut board.hands[index]) {
                say(&format!("  {:?} hand refused: {error}", source.handedness));
                continue;
            }
            let tracked = wxr::HandJoint::ALL
                .iter()
                .filter(|joint| board.hands[index].joint(**joint).is_some())
                .count();
            say(&format!(
                "  {:?} hand: {tracked} of {} joints tracked",
                source.handedness,
                wxr::HandJoint::COUNT,
            ));
            // On a value rather than a count, and only where there is a binding: the stub's numbers are
            // what a run at `?gpu` is a second run of the same page against.
            if board.gpu {
                hands::check_hand(&board.hands[index], source.handedness)?;
            }
            // A buzz is a request that has been sent and nothing more: the answer is whether the platform
            // took it, not whether a motor moved, which is a thing no frame can see.
            match session.pulse(source.id, 0.5, Duration::from_millis(100)) {
                Ok(()) => say(&format!("  {:?} buzz sent", source.handedness)),
                Err(error) => say(&format!(
                    "  {:?} would not buzz: {error}",
                    source.handedness
                )),
            }
        }
    }

    let handed_over = session.image_count();
    if let Err(error) = session.end(&mut board.frame) {
        return Err(format!("end failed: {error}"));
    }
    // The browser owns a frame only until the animation callback returns, and `end` is where this backend
    // gives it back. Everything that belonged to it stops being an answer then, so the next `begin` has
    // nothing stale to clear and a caller between frames is told there is no frame rather than given the
    // picture from the one before.
    if session.image_count() != 0 {
        return Err(format!(
            "the frame was ended and the session still hands over {} image(s)",
            session.image_count()
        ));
    }
    if session.image(0).is_some() {
        return Err("the frame was ended and its image is still there".into());
    }
    if session.depth(0).is_some() {
        return Err("the frame was ended and its depth is still there".into());
    }
    board.drawn += 1;
    say(&format!(
        "frame {} drawn ({handed_over} image(s) this session handed over)",
        board.drawn
    ));
    Ok(())
}
