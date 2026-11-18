//! The anchors a session without any answers, which is the default every backend begins from.
//!
//! The mock makes no anchors and keeps none between runs, so these are the core's own answers - and they are
//! the ones worth pinning: anchoring is opt-in, and a caller that asks a session without it has to be told
//! rather than handed a handle that never resolves or a stale list out of a vector it reused.

use super::*;
use crate::{Anchor, AnchorId};

#[test]
fn a_session_without_anchors_says_so() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");

    // No way to make one says so, rather than answering with a handle whose place never arrives.
    assert!(matches!(
        session.anchor(space, Pose::IDENTITY),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(session.anchor_pose(Anchor::new(0), space).unwrap(), None);

    // A handle is a slot in a table this session does not have, so it has no name either.
    assert_eq!(session.anchor_id(Anchor::new(0)), None);
    // A name from an earlier run is the same kind of question, and the answer is the core's "no runtime
    // keeps this for me" rather than a handle that would stay empty for good.
    assert!(matches!(
        session.restore_anchor(AnchorId::from_bytes([7; 16])),
        Err(Error::Unsupported(_))
    ));

    // The list is emptied before that refusal: a caller that keeps its vector across sessions and backends
    // must not read the last one's names as this one's answer to the same question.
    let mut names = vec![AnchorId::from_bytes([7; 16])];
    assert!(matches!(
        session.persistent_anchor_ids(&mut names),
        Err(Error::Unsupported(_))
    ));
    assert!(names.is_empty());

    // Letting go of a handle this session never handed out is not an error, and reaches no runtime.
    session.release_anchor(Anchor::new(0));
}
