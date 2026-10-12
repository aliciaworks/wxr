//! This backend, against the runtime the machine actually has.
//!
//! Every other check in this workspace is a comparison of one reading of an API against another: the
//! vocabulary suite against Servo's, the Apple declarations against an SDK mirror. This is the one that
//! asks the runtime. It runs on a machine with no headset - Monado has a simulated HMD and a null
//! compositor for exactly this - and it skips itself on a machine with no runtime at all, because a
//! backend that reports `Unavailable` has answered correctly and a test that fails for it would be
//! testing the machine.
//!
//! What it also does is run the frame loop, which is the part this file used to say it could not. A frame is
//! `xrWaitFrame`, and on a machine with no display it comes back with `should_render` false for as long as the
//! session is not on one - but OpenXR still wants its `xrBeginFrame` and `xrEndFrame` for that frame, and
//! leaving the pair open is a second `xrWaitFrame` that never returns. The backend closes the pair itself,
//! because the caller is told `Wait` and so never reaches `Session::end`; that is what makes this suite, and
//! `examples/headless.rs`, able to run to the end on a machine with a compositor that shows nothing.

// The crate under test is empty on a browser, so there is nothing here to name either - the same rule its
// `lib.rs` states, applied to the tests of it.
#![cfg(not(target_family = "wasm"))]

mod common;

use common::connect;
use wxr::{Session as _, SpaceKind};

#[test]
fn a_runtime_reports_what_it_is_before_a_session_exists() {
    let Some((_turn, backend, _session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // The extent is what a swapchain is made at, and a runtime with nothing to recommend would be handing
    // over a zero-sized image - which is not a size a renderer can draw into.
    let extent = backend.recommended_extent();
    assert!(
        extent.width > 0 && extent.height > 0,
        "the runtime recommended a {extent:?} image, which nothing can be drawn into"
    );

    // The Vulkan versions are asked for by name because this is the only backend that has a graphics API to
    // ask about: OpenXR has no wgpu binding, so a session is handed the handles the renderer made.
    let (min, max) = backend.requirements().expect("the graphics requirements");
    assert!(
        max >= min,
        "the runtime's Vulkan range is inverted: {min:?} above {max:?}"
    );
}

#[test]
fn a_session_reports_the_shape_of_its_images_and_what_it_is() {
    let Some((_turn, _, session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // A session that connected is a session, and the core's ladder has one rung for it. `Connecting` is
    // WebXR's asynchronous `requestSession`, which OpenXR does not have - its session arrives in the call.
    assert_eq!(session.state(), wxr::State::Ready);
    // Nothing is on a display before the frame loop begins, and this suite does not begin one.
    assert_eq!(session.visibility(), wxr::Visibility::Hidden);

    // The renderer owns the picture on this platform: it draws into the compositor's images, the same as
    // WebXR and unlike RealityKit, which draws the scene itself.
    assert_eq!(session.presentation(), wxr::Presentation::Composited);

    let images = session.images();
    assert!(
        images.extent.width > 0 && images.extent.height > 0,
        "the session's images are {images:?}, which nothing can be drawn into"
    );
    assert!(
        images.layers >= 1,
        "the session reports {} array layers, which is not a stereo pair or a mono image",
        images.layers
    );

    // One image, because this backend hands back the image the frame acquired - the one the compositor will
    // present - rather than the swapchain's first, which is a different image on most frames.
    assert_eq!(session.image_count(), 1);
    // And no image before a frame has acquired one, which is the same statement: the handle is the frame's.
    assert!(
        session.image(0).is_none(),
        "a session with no frame has an image, which nobody acquired"
    );
}

#[test]
fn every_capability_bit_is_something_the_session_will_answer_for() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    let features = session.features();

    // A quad layer is in the core specification, so a composited session that claims no quad is claiming it
    // cannot present a rectangle - and the call below is how that would be found out.
    let space = session.space(SpaceKind::Local).expect("a local space");
    let layer = session
        .layer(
            space,
            wxr::LayerShape::Quad {
                width: 1.0,
                height: 1.0,
            },
            wxr::Extent2d::new(64, 64),
        )
        .expect("a quad layer, which is in the core specification");
    session.release_layer(layer);

    // And a shape the bits do not claim is refused rather than quietly made into a quad. The bit and the
    // answer are the same answer, read in two places, which is what makes checking one against the other
    // mean something.
    for (shape, bit) in [
        (
            wxr::LayerShape::Cylinder {
                radius: 1.0,
                central_angle: 1.0,
                aspect: 0.5,
            },
            wxr::Features::LAYER_CYLINDER,
        ),
        (
            wxr::LayerShape::Equirect {
                radius: 1.0,
                central_horizontal: 1.0,
                upper_vertical: 0.5,
                lower_vertical: 0.5,
            },
            wxr::Features::LAYER_EQUIRECT,
        ),
        (wxr::LayerShape::Cube, wxr::Features::LAYER_CUBE),
    ] {
        let made = session.layer(space, shape, wxr::Extent2d::new(128, 128));
        assert_eq!(
            made.is_ok(),
            features.contains(bit),
            "a {} layer was {} but {bit:?} said {}",
            shape.name(),
            if made.is_ok() { "made" } else { "refused" },
            features.contains(bit),
        );
        if let Ok(made) = made {
            session.release_layer(made);
        }
    }
}

#[test]
fn a_space_of_each_kind_is_a_space_or_says_it_cannot_be() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // Every kind is asked for, because which of them a runtime has is the runtime's - `LocalFloor` is one
    // extension, `BoundedFloor` needs a boundary the user walked, and a headset that has neither is not
    // broken. What the core insists on is the answer: a space, or `NoSpace` naming the kind that was not
    // there.
    for kind in [
        SpaceKind::Viewer,
        SpaceKind::Local,
        SpaceKind::LocalFloor,
        SpaceKind::BoundedFloor,
        SpaceKind::Unbounded,
    ] {
        match session.space(kind) {
            Ok(space) => assert_eq!(space.kind, kind),
            Err(wxr::Error::NoSpace(requested)) => assert_eq!(requested, kind),
            Err(other) => panic!("a {kind:?} space failed with {other}, which is not NoSpace"),
        }
    }
}

#[test]
fn an_offset_space_is_another_space_of_the_same_kind() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    let base = session.space(SpaceKind::Local).expect("a local space");
    let moved = session
        .offset_space(
            base,
            wxr::Pose {
                position: wxr::glam::Vec3::new(0.5, 1.5, -2.0),
                orientation: wxr::glam::Quat::IDENTITY,
            },
        )
        .expect("an offset of a space that exists");

    // The same kind, because that is what keeps it tracking the room rather than becoming a fixed point:
    // the offset is a pose inside the base, not a new kind of origin.
    assert_eq!(moved.kind, base.kind);
    // And a different space, so that the two can be handed to the renderer separately.
    assert_ne!(moved.id(), base.id());
}

#[test]
fn a_boundary_belongs_to_a_bounded_space_and_is_a_closed_loop() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // A boundary is a fact about a bounded space and about nothing else: `BoundedFloor` is the space whose
    // definition includes the room the user walked, and `Local` is an origin with no claim on a wall. So
    // asking the second is an outline with no points, which is not an error - a floor with no wall drawn
    // around it is still a floor.
    let local = session.space(SpaceKind::Local).expect("a local space");
    let mut outline = vec![wxr::glam::Vec2::new(9.0, 9.0)];
    session
        .bounds(local, &mut outline)
        .expect("a boundary asked of a space with none");
    assert!(
        outline.is_empty(),
        "a local space has a boundary: {outline:?}"
    );

    // The bounded one, whether or not this machine's runtime has walked a room. A simulated head has no room
    // to walk, so `None` is the likely answer - and it is an answer rather than a failure, which is the whole
    // point of the shape: empty means there is no wall, not that something went wrong.
    let bounded = session
        .space(SpaceKind::BoundedFloor)
        .expect("a bounded floor space");
    session
        .bounds(bounded, &mut outline)
        .expect("a boundary asked of a bounded space");

    // Which is what a runtime without a room answers, and there is nothing further to check.
    if outline.is_empty() {
        return;
    }

    // Four corners, because the room OpenXR tracks is a rectangle - and written out as the closed loop the
    // core asks for, so a caller can walk it without the last point having to repeat the first.
    assert_eq!(
        outline.len(),
        4,
        "a rectangle is not four corners: {outline:?}"
    );

    // And it is a rectangle: opposite corners are negatives of each other, which is what centred on the
    // origin means. A runtime that reports a zero-sized room would fail here, and should.
    assert_eq!(outline[0], -outline[2]);
    assert_eq!(outline[1], -outline[3]);
}

#[test]
fn a_refresh_rate_is_offered_or_the_call_is_a_silence() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // Asking is not setting: a runtime that will not take a refresh rate answers the enumerate with an error
    // this backend logs rather than a failure, because a session with a fixed display is still a session.
    // So whatever comes back, the answer has to be rates a display could actually run at.
    let mut rates = Vec::new();
    session.refresh_rates(&mut rates);
    for rate in &rates {
        assert!(
            *rate > 0.0 && *rate < 1000.0,
            "the runtime offered a {rate} Hz display, which is not a rate"
        );
    }

    // The bit and the answer are the same answer again: the capability is a promise that this call has
    // something to say, and a runtime without the extension is one where the list stays empty.
    assert_eq!(
        session.features().contains(wxr::Features::REFRESH_RATE),
        !rates.is_empty(),
        "{:?} and a list of {} rates disagree",
        wxr::Features::REFRESH_RATE,
        rates.len(),
    );

    // And a rate is asked for without a failure escaping, whichever the runtime makes of it: the worst
    // outcome is a logged refusal, since an app that wants 90 Hz on a 60 Hz panel has asked for the
    // impossible and does not need the frame loop to end over it.
    let rate = rates.first().copied().unwrap_or(90.0);
    session.set_refresh_rate(rate);
}

#[test]
fn inputs_on_a_runtime_with_no_controllers_are_none_and_not_a_failure() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    let space = session.space(SpaceKind::Local).expect("a local space");

    // A runtime that will not take the action set is a session with no inputs rather than a session that
    // failed - which is what a machine with a head and no controllers is. The simulated head here is exactly
    // that, so this is the branch rather than the exception; the assertion is that it is quiet about it.
    let mut sources = Vec::new();
    session
        .inputs(space, &mut sources)
        .expect("inputs asked of a session with none");

    // No source names a hand that is not there. A runtime with controllers reports them here instead, and
    // the same assertion holds: a source is a thing with a hand on it.
    for source in &sources {
        assert!(
            matches!(
                source.handedness,
                wxr::Handedness::Left | wxr::Handedness::Right
            ),
            "a source came back with no hand: {source:?}"
        );
    }

    // And a hand read for an id that exists comes back as an answer - empty when nothing is tracked, which
    // on this machine is always. What it must not do is fail: `hand` is a read of a frame's worth of
    // skeleton, and a runtime with no hands to give has nothing to read.
    let mut hand = wxr::Hand::default();
    session
        .hand(wxr::InputId::new(0), space, &mut hand)
        .expect("a hand asked of a session with none");
}

#[test]
fn a_frame_before_the_session_is_begun_is_a_frame_to_wait_for() {
    // Its own session, and one that is never polled: polling is what begins a session, and `xrWaitFrame`
    // after that is a wait for a display this machine does not have.
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // This is the deadlock the trait is written against, named in `begin`'s own documentation: a runtime is
    // not asked to wait for a frame until the session has been begun, and the event that says to begin
    // arrives on the poll the caller is not making while it waits. A session that has not begun answers
    // `Wait` - a frame to begin another of - rather than blocking on one that will never come.
    let mut frame = wxr::Frame::default();
    session
        .begin(std::time::Duration::ZERO, &mut frame)
        .expect("a frame before the session is begun");
    assert!(
        !frame.is_render(),
        "a session that has not begun is drawing, which is a frame nobody waited for"
    );
    assert_eq!(frame.views().len(), 0);
}

#[test]
fn a_frame_the_runtime_has_nothing_to_draw_in_is_still_a_frame_that_comes_back() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // A frame is submitted in a space, and this backend builds its projection layer in the first space the
    // session has - so a session with none has a frame to wait for and nowhere to hand one back to. `Local`
    // is the one space the specification says every runtime must make.
    let Ok(space) = session.space(SpaceKind::Local) else {
        eprintln!("this runtime makes no local space; there is no frame to submit one in");
        return;
    };

    // The frame loop, on a machine whose compositor never puts the session on a display: `xrWaitFrame`
    // answers with nothing to draw, the caller is told `Wait`, and `Session::end` is not called - which is
    // what every caller in the workspace does with a frame it has nothing to draw in. OpenXR still wants
    // that frame begun and ended, and the backend is the only one who can do it. Without that, the second
    // `xrWaitFrame` below never returns, and this test hangs instead of failing.
    let mut frame = wxr::Frame::default();
    let mut drawn = 0;
    for _ in 0..120 {
        while session.poll().is_some() {}
        session
            .begin(std::time::Duration::ZERO, &mut frame)
            .expect("a frame of a begun session");
        if !frame.is_render() {
            continue;
        }
        session
            .views(space, &mut frame)
            .expect("the views of a frame that is being drawn");
        session.end(&mut frame).expect("a frame handed back");
        drawn += 1;
        if drawn >= 3 {
            break;
        }
    }

    // Three, because one proves nothing about the *second* `xrWaitFrame`, which is where an unpaired loop
    // stops - and because a runtime that has begun a session it will never show is answered by the loop
    // coming back at all, which is the thing this test is here to notice.
    assert_eq!(
        drawn, 3,
        "the runtime gave {drawn} frame(s) to draw out of the 120 asked for"
    );
}

#[test]
fn the_backend_is_reachable_through_the_core_seam() {
    let Some((_turn, _, session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    // The seam is the whole design: the core is a subset, and a program that needs the part OpenXR has and
    // the core does not reaches it by naming this type - no method was added to the core for it, and nothing
    // on the way in knew which backend it was asking for.
    assert!(session.as_backend::<wxr_openxr::OpenXrSession>().is_some());

    // Which is how OpenXR's own state is read, out of a session typed as the core's. The two ladders are
    // not the same ladder: the core has `Ready` because a session exists, and OpenXR has `IDLE` because a
    // session that has not been begun shows nothing. A program that needs to know which of OpenXR's states
    // it is on asks, rather than inferring it from the core's one rung.
    let state = session
        .as_backend::<wxr_openxr::OpenXrSession>()
        .expect("the backend, named again")
        .openxr_state();
    assert_eq!(
        state,
        openxr::SessionState::IDLE,
        "a session that has not been polled is not IDLE to OpenXR, which is the ladder it is climbed"
    );
}
