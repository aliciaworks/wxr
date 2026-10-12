//! The four ways a runtime can say what the room is, asked of one.
//!
//! Split out of `runtime.rs` for the size rule, and kept as its own suite because it is the question the
//! layers are asked: the capability bit in `Session::features` and the answer the matching method gives are
//! the same fact read in two places, and a bit that says yes where the method says no is a promise the
//! session is not keeping.
//!
//! It needs a connected session, so it shares [`common::connect`] and the lock beside it.

#![cfg(not(target_family = "wasm"))]

mod common;

use common::connect;
use wxr::{Session as _, SpaceKind};

/// The four ways a runtime can say what the room is, checked the same way the layers are: the bit in
/// `features()` and the answer the matching method gives are the same fact read in two places.
///
/// The layer test above can check both directions because a layer is either made or refused and that is
/// the whole answer. These are not all like that - `Session::depth` is `Option` on a runtime that has one
/// and has not been asked yet - so what is checked here is the direction that can be checked exactly: a
/// bit **not** set means the call refuses, and refuses with `Unsupported` rather than an error about
/// something else. A bit set for a method that says the platform cannot do it is a promise the session is
/// not keeping, which is the thing `Features` exists to prevent.
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
    // But if the runtime does say it has planes, the first step of a pass has to be one that could produce
    // them - an empty list is a pass in flight, and a pass that was never begun is an empty list forever.
    if features.contains(wxr::Features::PLANES) {
        session.planes(space, &mut found).expect("a plane pass");
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
        // A just-made anchor is one the runtime has not finished with, so `None` is the answer the core
        // documents for it. What must not happen is an error from a handle this session handed out.
        assert!(
            session.anchor_pose(anchor, space).is_ok(),
            "anchor_pose() on an anchor the session made refused"
        );
        session.release_anchor(anchor);
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
        let mut hits = Vec::new();
        assert!(
            session.hits(source, space, &mut hits).is_ok(),
            "hits() on a source the session made refused"
        );
    }

    // Depth is `Option` even where it exists - the provider is started at the first ask and the first ask
    // takes no image - so only the direction that is exact is checked: a session with no depth says so
    // from both of its calls.
    if !features.contains(wxr::Features::DEPTH) {
        assert!(
            session.depth(0).is_none(),
            "depth() gave something but DEPTH was not set"
        );
        assert!(
            session.depth_sensing().is_none(),
            "depth_sensing() answered but DEPTH was not set"
        );
    }
}
