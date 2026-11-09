//! The four ways a runtime can say what the room is, asked of one and given a turn to answer.
//!
//! Split out of `runtime.rs` for the size rule, and kept as its own suite because it is the question the
//! layers are asked: the capability bit in `Session::features` and the answer the matching method gives are
//! the same fact read in two places, and a bit that says yes where the method says no is a promise the
//! session is not keeping.
//!
//! Two of the four are asynchronous in a way the core's shape hides - an anchor exists when an event says
//! so, and a raycaster exists when a future is polled to readiness - so this suite also runs the part of a
//! frame loop that delivers both. See [`turn`].
//!
//! It needs a connected session, so it shares [`common::connect`] and the lock beside it.

#![cfg(not(target_family = "wasm"))]

mod common;

use std::time::Duration;

use common::connect;
use wxr::{Session as _, SpaceKind};

/// How many turns a runtime gets before it is taken not to be going to answer.
///
/// A bound on the waiting and not an expectation of it: every assertion below holds for a runtime that never
/// answers, which is the runtime this runs against almost everywhere - Monado has none of these four
/// extensions and no CI runner has an XR runtime at all. The count is what makes the suite mean something on
/// the machines that *can* answer, without making it fail on the ones that cannot.
const TURNS: usize = 60;

/// One turn of a frame loop: everything the runtime has to say, then the gap a frame would have been.
///
/// This is the part of a frame loop that does not need a compositor, and it is the part that turns the two
/// asynchronous extensions from "exists" into "answered". A runtime posts `SpatialAnchorCreateCompleteFB`
/// when *it* has finished making an anchor, and `xrPollEvent` is the only way to receive one; a future moves
/// only when something calls `xrPollFutureEXT`, and the session does that on every poll. The sleep is what
/// gives the runtime a chance to finish anything at all - a loop that never yields to it is a loop that
/// polls a future which is still pending because nothing has run.
fn turn(session: &mut wxr_openxr::OpenXrSession) {
    while session.poll().is_some() {}
    std::thread::sleep(Duration::from_millis(16));
}

/// The four ways a runtime can say what the room is, checked the same way the layers are: the bit in
/// `features()` and the answer the matching method gives are the same fact read in two places.
///
/// The layer test can check both directions because a layer is either made or refused and that is the whole
/// answer. These are not all like that - `Session::depth` is `Option` on a runtime that has one and has not
/// been asked yet - so the direction that can be checked exactly is the one checked exactly: a bit **not**
/// set means the call refuses, and refuses with `Unsupported` rather than an error about something else. A
/// bit set for a method that says the platform cannot do it is a promise the session is not keeping, which
/// is the thing `Features` exists to prevent.
///
/// Where a runtime does have one, the method is asked and pumped rather than only asked, so that the
/// asynchronous halves are reached on hardware and are simply absent here. What is asserted about them is
/// still only what is true on every runtime - see each block.
#[test]
fn a_world_modelling_bit_is_the_same_fact_as_its_method() {
    let Some((_turn, _, mut session)) = connect() else {
        eprintln!("no OpenXR runtime on this machine; nothing to connect to");
        return;
    };

    let features = session.features();
    let space = session.space(SpaceKind::Local).expect("a local space");
    let pose = wxr::Pose::IDENTITY;

    // Surfaces are the one that is never a refusal: the core's default is an empty list rather than an
    // error, because a session that has not found a surface and a backend with no way to look for one are
    // the same thing to a caller who is only asking whether there is a table here.
    let mut found = Vec::new();
    assert!(
        session.planes(space, &mut found).is_ok(),
        "planes() refused, and it is one of the calls that has nothing to refuse"
    );
    // But if the runtime does say it has planes, the first steps of a pass have to be ones that could
    // produce them - an empty list is a pass in flight, and a pass that was never begun is an empty list
    // forever. A pass takes more than one turn by design, which is what `MAX_PLANES` and the state machine
    // in `planes.rs` are about, so this drives several and asks only that none of them is an error.
    if features.contains(wxr::Features::PLANES) {
        for _ in 0..TURNS {
            turn(&mut session);
            session.planes(space, &mut found).expect("a plane pass");
        }
    }

    // Anchors and hit-test sources are made, so they are refused outright without the extension - and with
    // it they are a handle, which is what makes both directions checkable.
    let anchor = session.anchor(space, pose);
    assert_eq!(
        anchor.is_ok(),
        features.contains(wxr::Features::ANCHORS),
        "anchor() was {} but ANCHORS said {}",
        if anchor.is_ok() { "made" } else { "refused" },
        features.contains(wxr::Features::ANCHORS),
    );
    if let Ok(anchor) = anchor {
        // The runtime answers by event, and this backend only learns where an anchor is once `poll` has
        // handed that event over - so a test that asked once and gave up would be testing the asking and
        // not the making. This is the frame loop the core's shape assumes, done by hand.
        for _ in 0..TURNS {
            turn(&mut session);
            if session
                .anchor_pose(anchor, space)
                .is_ok_and(|at| at.is_some())
            {
                break;
            }
        }

        // `None` is a legitimate answer even here, and it is three things the caller cannot tell apart: a
        // runtime still making the anchor, one that has lost it, and one that was released. So what is
        // asserted is the shape of the answer and not its value - a handle this session handed out does not
        // produce an error, whatever the runtime has to say about where it is.
        let located = session.anchor_pose(anchor, space);
        assert!(
            located.is_ok(),
            "anchor_pose() on an anchor this session made refused: {located:?}"
        );

        // And a released anchor is the one direction of that answer this backend owns rather than the
        // runtime: releasing clears the slot, so the answer is `None` because this session says so.
        session.release_anchor(anchor);
        assert!(
            session
                .anchor_pose(anchor, space)
                .expect("a released handle is still a handle")
                .is_none(),
            "a released anchor was still locatable"
        );
    }

    let source = session.hit_test_source(space);
    assert_eq!(
        source.is_ok(),
        features.contains(wxr::Features::HIT_TEST),
        "hit_test_source() was {} but HIT_TEST said {}",
        if source.is_ok() { "made" } else { "refused" },
        features.contains(wxr::Features::HIT_TEST),
    );
    if let Ok(source) = source {
        // Making a raycaster is asynchronous and a future only moves when something polls it, so this is
        // the other half of asking for one: a source that is never pumped is a source that can never
        // answer, on any runtime, however long it is held.
        for _ in 0..TURNS {
            turn(&mut session);
        }

        let mut hits = Vec::new();
        assert!(
            session.hits(source, space, &mut hits).is_ok(),
            "hits() on a source the session made refused"
        );
        // Meeting nothing is a normal answer, and it is the same empty list as a raycaster that is still
        // being made - which is what the core says to expect. What is not allowed is two: this backend
        // casts one ray and the extension answers with one surface, so a second entry would be this
        // backend's invention rather than the runtime's.
        assert!(
            hits.len() <= 1,
            "one ray met {} surfaces, and the extension has no way to say that",
            hits.len()
        );

        // And a released source takes the same direction the released anchor took: the slot is this
        // backend's, so what changes is one it answers rather than one it cannot. The handle is still the
        // handle this session handed out - it stops being an error either way - and `hits` on it is empty
        // because the ray is no longer measured from anywhere, which is what a caller is told.
        session.release_hit_test_source(source);
        assert!(
            session.hits(source, space, &mut hits).is_ok(),
            "hits() on a released source refused instead of answering nothing"
        );
        assert!(
            hits.is_empty(),
            "a released source went on reporting {} surfaces",
            hits.len()
        );
    }

    // Depth is `Option` even where it exists, because the provider is made at connect and *started* at the
    // first ask - so the ask is made here either way, and what it proves depends on which runtime this is.
    //
    // No image comes back from it even on a headset: the runtime hands one over only between a frame's
    // begin and its end, and this suite has neither, so the acquire is refused and `depth` answers `None`.
    // What the ask does prove is the start, which is the whole of what the laziness is for.
    let first = session.depth(0);
    if features.contains(wxr::Features::DEPTH) {
        assert_eq!(
            session.depth_sensing().and_then(|sensing| sensing.active),
            Some(true),
            "asking for depth did not start the provider"
        );
    } else {
        assert!(
            first.is_none(),
            "depth() gave something but DEPTH was not set"
        );
        assert!(
            session.depth_sensing().is_none(),
            "depth_sensing() answered but DEPTH was not set"
        );
    }
}
