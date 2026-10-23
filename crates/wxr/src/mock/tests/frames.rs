//! The frame loop, on the mock: eyes, prediction, and what a reused `Frame` forgets.

use super::*;

#[test]
fn a_frame_has_an_eye_a_side_an_interpupillary_distance_apart() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();

    assert!(frame.is_render());
    // The head is where the eyes are centred, and the views are around it.
    assert_eq!(frame.viewer, Pose::IDENTITY);
    assert_eq!(frame.views().len(), 2);
    let (left, right) = (&frame.views()[0], &frame.views()[1]);
    assert_eq!(left.eye, Eye::Left);
    assert_eq!(right.eye, Eye::Right);
    let apart = (right.pose.position - left.pose.position).length();
    assert!((apart - IPD).abs() < 1e-6, "{apart}");
    // The eyes are layers of one image, not two images.
    assert_ne!(left.layer, right.layer);
}

#[test]
fn a_new_frame_does_not_carry_the_last_one_head() {
    // A frame loop reuses one `Frame`, and `begin` is what makes it this frame's. A `Wait` or a `views`
    // that could not locate the eyes leaves nothing in it - not the head from the frame before.
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();
    frame.viewer = Pose {
        position: Vec3::Y,
        orientation: Quat::IDENTITY,
    };
    session.begin(Duration::ZERO, &mut frame).unwrap();
    assert!(frame.views().is_empty(), "last frame's eyes are gone");
    assert_eq!(frame.viewer, Pose::IDENTITY, "and last frame's head");
}

#[test]
fn the_prediction_is_ahead_of_now_not_behind_it() {
    let mut session = running();
    let mut frame = Frame::default();
    let now = Duration::from_secs(3);
    session.begin(now, &mut frame).unwrap();
    assert!(frame.predicted_display_time > now);
}

#[test]
fn a_stopped_session_asks_to_exit() {
    let mut session = MockBackend {
        states: vec![State::Ready, State::Ended],
        ..Default::default()
    }
    .connect((), SessionMode::ImmersiveVr)
    .unwrap();
    session.poll();
    session.poll();
    assert_eq!(session.state(), State::Ended);
    assert!(!session.state().is_alive());
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    assert_eq!(frame.state, FrameState::Exit);
}
