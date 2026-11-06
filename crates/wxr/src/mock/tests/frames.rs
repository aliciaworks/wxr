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
fn asking_for_views_twice_replaces_the_eyes() {
    // A caller may ask for the same frame's views more than once - a retry, or a second space - and the
    // frame holds what the backend last said rather than every answer it has ever given. Four eyes are not
    // a stereo frame.
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();
    assert_eq!(frame.views().len(), 2);
    assert_eq!(frame.views()[0].eye, Eye::Left);
    assert_eq!(frame.views()[1].eye, Eye::Right);
}

#[test]
fn a_views_call_the_space_refused_drops_the_eyes() {
    // The mock is the backend a caller develops against, so a failed call has to leave the frame in the same
    // shape a real one does: no eyes, rather than the pair the last successful call put there. Otherwise a
    // caller that ignores the error presents a frame from a space it was just told about.
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();
    assert_eq!(frame.views().len(), 2);
    let head = ReferenceSpace::new(SpaceKind::Local, 0);
    assert!(matches!(
        session.views(head, &mut frame),
        Err(Error::NoSpace(SpaceKind::Local))
    ));
    assert!(frame.views().is_empty(), "the refused call's eyes are gone");
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

#[test]
fn a_picture_belongs_to_a_frame_that_is_open() {
    // The handle is the frame's: a compositor gives one for the frame and takes it back at the end. A mock
    // that answered outside that window would let a renderer keep a texture a real backend has withdrawn.
    let mut session = running();
    assert!(session.image(0).is_none(), "before any frame");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    assert!(session.image(0).is_some(), "the frame is open");
    session.end(&mut frame).unwrap();
    assert!(session.image(0).is_none(), "and it is over");
}

#[test]
fn a_layer_picture_belongs_to_a_frame_that_is_open() {
    let mut session = with_layers();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let layer = session
        .layer(
            space,
            LayerShape::Quad {
                width: 1.0,
                height: 1.0,
            },
            Extent2d::new(64, 64),
        )
        .expect("a quad layer");
    assert!(session.layer_image(layer).is_none(), "before any frame");
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    assert!(session.layer_image(layer).is_some(), "the frame is open");
    session.end(&mut frame).unwrap();
    assert!(session.layer_image(layer).is_none(), "and it is over");
}
