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

    // Asked before the floor is, so that a check of the answer runs on the tick after the request settles.
    if !board.refusal_checked
        && let Some(unbounded) = board.unbounded
    {
        board.refusal_checked = true;
        match session.offset_space(unbounded, wxr::Pose::IDENTITY) {
            Err(wxr::Error::NoSpace(wxr::SpaceKind::Unbounded)) => {
                say("an offset of the space the browser refused is NoSpace");
            }
            Err(error) => return Err(format!("an offset of a refused space answered {error}")),
            Ok(_) => return Err("an offset of the space the browser refused was accepted".into()),
        }
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
        // The stub has no unbounded space, so this request settles without one. Nothing is drawn in it; it is
        // here for the offset check above, which is the one thing a refused request has to get right.
        match session.space(wxr::SpaceKind::Unbounded) {
            Ok(space) => board.unbounded = Some(space),
            Err(error) => say(&format!(
                "the unbounded space was refused outright: {error}"
            )),
        }
    }
    // The refresh rate is a `float` on the way in and a promise on the way out, and both halves are the
    // browser's: the list it offers, the rate it took, and the two values that are not numbers of hertz at
    // all. Asked once, because the list is the session's rather than the frame's - and read back off the stub,
    // because a request is not the answer. It is before the frame for that reason: no frame is needed.
    if !board.refresh_rate_checked {
        board.refresh_rate_checked = true;
        if !session.features().contains(wxr::Features::REFRESH_RATE) {
            return Err(format!(
                "the stub offers frame rates, where the session reports {:?}",
                session.features()
            ));
        }
        // Seeded with a caller's own value, because the contract is that a list is appended to: a backend
        // that replaced the vector would drop the 60 here and look right in a run that passed an empty one.
        let mut rates = vec![60.0];
        session.refresh_rates(&mut rates);
        if rates != [60.0, 72.0, 90.0, 120.0] {
            return Err(format!(
                "the refresh rates were {rates:?}, where the stub's list is 72, 90 and 120 after the caller's 60"
            ));
        }
        session.set_refresh_rate(f32::NAN);
        session.set_refresh_rate(f32::INFINITY);
        if frame_rate_requests().unwrap_or(0) != 0 {
            return Err(format!(
                "a NaN and an infinity asked for a refresh rate, where neither is a number of hertz: {} request(s)",
                frame_rate_requests().unwrap_or(u32::MAX)
            ));
        }
        // The rate the display offers, which the stub's own `frameRate` says it is - and then one it does
        // not, whose promise rejection this backend has to read rather than leave unhandled on the page.
        session.set_refresh_rate(90.0);
        if frame_rate() != Some(90.0) {
            return Err(format!(
                "a 90 Hz request landed as {:?}, where the display offers 90",
                frame_rate()
            ));
        }
        session.set_refresh_rate(60.0);
        if frame_rate() != Some(90.0) {
            return Err(format!(
                "a rate the display does not offer changed it to {:?}",
                frame_rate()
            ));
        }
        if frame_rate_requests() != Some(2) {
            return Err(format!(
                "{} refresh-rate request(s) reached the display, where only the two finite ones should have",
                frame_rate_requests().unwrap_or(u32::MAX)
            ));
        }
        say("the display offers 72, 90 and 120 Hz: 90 was taken and 60 was refused");
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
    // The stub's `XRFrame` predicts 12.3455 s and its callback's own timestamp is never that, so this is true
    // only if `begin` read the frame's clock rather than the animation callback's.
    let predicted = board.frame.predicted_display_time.as_secs_f64();
    if (predicted - 12.3455).abs() > 1e-9 {
        return Err(format!(
            "the frame's predicted display time is {predicted} s, where the stub predicts 12.3455 s"
        ));
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
    // A `views` retry is a new answer rather than an addition to the last one, so a space the browser refused
    // has to leave the frame without eyes *and* without the picture those eyes were drawn through. Asked once,
    // after the image checks above and before anything else reads the frame, and the real space is asked again
    // below so that this tick still draws.
    if board.gpu && board.checked && !board.retry_checked {
        board.retry_checked = true;
        if let Some(unbounded) = board.unbounded {
            if let Err(error) = session.views(unbounded, &mut board.frame) {
                return Err(format!("views of a refused space answered {error}"));
            }
            if !board.frame.views().is_empty() {
                return Err(format!(
                    "a views call for a refused space left {} eye(s) in the frame",
                    board.frame.views().len()
                ));
            }
            let meta = session.images();
            if session.image_count() != 0
                || session.image(0).is_some()
                || meta.format != wxr::ColorFormat::Unknown
                || meta.extent != wxr::Extent2d::default()
            {
                return Err(format!(
                    "a views call with no eyes left a picture behind: {} image(s), {:?} at {:?}",
                    session.image_count(),
                    meta.format,
                    meta.extent
                ));
            }
            say("a views retry with no eyes took the picture with it");
            if let Err(error) = session.views(space, &mut board.frame) {
                return Err(format!("the floor space's views, asked again: {error}"));
            }
            if board.frame.views().len() != 2 || session.image_count() != 1 {
                return Err(format!(
                    "the retry left {} eye(s) and {} image(s), where the frame had two and one",
                    board.frame.views().len(),
                    session.image_count()
                ));
            }
        }
    }
    // The viewport scale is a WebIDL `double`, so a value the browser cannot convert is a `TypeError` thrown
    // through the wasm frame - the page stops rather than drawing. Ask with the three values that are not a
    // scale and with one that is: NaN must leave the last request alone, an infinity must clamp to one, and a
    // half must arrive. The answer is read back off the stub, because the request is not the answer.
    if !board.viewport_scale_checked && !board.frame.views().is_empty() {
        board.viewport_scale_checked = true;
        session.request_viewport_scale(0, Some(f32::NAN));
        if viewport_scale().is_some() {
            return Err("a NaN viewport scale was taken for a request".into());
        }
        session.request_viewport_scale(0, Some(f32::INFINITY));
        if viewport_scale() != Some(1.0) {
            return Err(format!(
                "an infinite viewport scale landed as {:?}, where the clamp is one",
                viewport_scale()
            ));
        }
        session.request_viewport_scale(0, Some(0.5));
        if viewport_scale() != Some(0.5) {
            return Err(format!(
                "a half viewport scale landed as {:?}",
                viewport_scale()
            ));
        }
        say("a NaN viewport scale was dropped, an infinite one clamped to one, and a half arrived");
    }
    // The layers the timer handed over, read back in a frame: an image per layer, and the shapes' own numbers
    // as the setters landed them. Read once, because a working layer is the same picture every frame.
    layers::check_frame(board, session)?;
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

    // A handle this session never handed out is refused rather than answered with nothing. The difference is
    // the whole reason `NoSpace` exists: a caller told "no planes, no hand, no wall" believes the room, while
    // a caller told `NoSpace` knows it asked the wrong session. `bounds` is the cheap one - it writes only the
    // caller's vector - and the floor is asked after it so that the valid half is shown to still answer.
    if !board.forged_space_checked {
        board.forged_space_checked = true;
        let forged = wxr::ReferenceSpace::new(wxr::SpaceKind::LocalFloor, 4096);
        let mut outline = Vec::new();
        match session.bounds(forged, &mut outline) {
            Err(wxr::Error::NoSpace(wxr::SpaceKind::LocalFloor)) => {}
            other => {
                return Err(format!(
                    "a space handle this session never made was answered as {other:?}, where NoSpace is the \
                     answer"
                ));
            }
        }
        if let Err(error) = session.bounds(space, &mut outline) {
            return Err(format!("the floor space's bounds: {error}"));
        }
        say("a space handle this session never made was refused, and the floor's was answered");
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
