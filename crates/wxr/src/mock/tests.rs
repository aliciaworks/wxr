//! What a session is, exercised on the mock one - which is the point of having it.

use super::*;

/// Drive the mock the way an app would: poll to `Visible`, then run frames.
fn running() -> MockSession {
    let mut session = MockBackend::default()
        .connect((), SessionMode::ImmersiveVr)
        .expect("the mock connects");
    while let Some(event) = session.poll() {
        if let Event::VisibilityChanged(Visibility::Visible) = event {
            break;
        }
    }
    session
}

#[test]
fn the_lifecycle_and_the_visibility_are_two_ladders() {
    let mut session = MockBackend::default()
        .connect((), SessionMode::ImmersiveVr)
        .unwrap();
    assert_eq!(session.state(), State::Connecting);
    assert_eq!(session.visibility(), Visibility::Hidden);

    // The session arrives, which is the whole of the lifecycle ladder in WebXR's terms...
    session.poll();
    assert_eq!(session.state(), State::Ready);
    assert!(!session.state().is_connecting());
    assert!(session.state().is_alive());

    // ...and then it is shown, in rungs of its own.
    session.poll();
    assert_eq!(session.visibility(), Visibility::Hidden);
    session.poll();
    assert_eq!(session.visibility(), Visibility::Visible);
    assert!(session.visibility().can_render());
    session.poll();
    assert_eq!(session.visibility(), Visibility::VisibleBlurred);
    assert!(
        session.visibility().can_render(),
        "blurred is still on a display, and a frame not drawn is a hole in it"
    );
    assert!(session.poll().is_none());
}

#[test]
fn the_escape_hatch_hands_back_the_backends_own_type() {
    let mut session = MockBackend::default()
        .connect((), SessionMode::ImmersiveVr)
        .unwrap();

    // The core's type says one thing...
    assert_eq!(session.state(), State::Connecting);
    // ...and the backend's own type is reachable through the seam, which is what a program needs when
    // the platform has something the core has decided not to say.
    let same: &MockSession = session
        .as_backend::<MockSession>()
        .expect("the mock is this backend");
    assert_eq!(same.state(), State::Connecting);

    // A different backend's type is `None` - not a panic, and not a lie.
    assert!(session.as_backend::<MockBackend>().is_none());

    // And the mutable half, which is how an app sets the platform's own knobs.
    let mutable: &mut MockSession = session
        .as_backend_mut::<MockSession>()
        .expect("the mock is this backend");
    mutable.poll();
    assert_eq!(session.state(), State::Ready);
}

#[test]
fn a_press_is_a_start_an_end_and_a_selection() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut sources = Vec::new();
    session.inputs(space, &mut sources).unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].id, InputId::new(0));
    assert_eq!(sources[0].handedness, Handedness::Left);
    assert_eq!(sources[1].id, InputId::new(1));
    assert_eq!(sources[0].target_ray_mode, TargetRayMode::TrackedPointer);

    let id = sources[1].id;
    // Nothing else is news, so the three below are the press and only the press.
    while session.poll().is_some() {}
    session.press(id);
    assert_eq!(session.poll(), Some(Event::SelectStart(id)));
    assert_eq!(session.poll(), Some(Event::SelectEnd(id)));
    assert_eq!(session.poll(), Some(Event::Select(id)));
    assert_eq!(session.poll(), None);
}

#[test]
fn a_recentered_space_is_stale_and_says_so() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    while session.poll().is_some() {}
    session.reset(space);
    assert_eq!(session.poll(), Some(Event::Reset(space)));
}

#[test]
fn a_space_with_no_boundary_has_an_empty_one() {
    let mut session = running();
    let space = session
        .space(SpaceKind::BoundedFloor)
        .expect("a bounded floor");
    let mut bounds = Vec::new();
    session.bounds(space, &mut bounds).unwrap();
    assert!(bounds.is_empty());
}

#[test]
fn a_backend_without_depth_sensing_answers_none() {
    let mut session = running();
    assert!(session.depth(0).is_none());
    assert_eq!(session.features(), crate::Features::NONE);

    // Anchors are the same shape: a backend that cannot make one says so rather than handing back a name that
    // never resolves.
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    assert!(matches!(
        session.anchor(space, Pose::IDENTITY),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        session.anchor_pose(crate::Anchor::new(0), space).unwrap(),
        None
    );
}

#[test]
fn a_backend_without_light_estimation_says_so() {
    let mut session = running();
    assert!(matches!(session.light_probe(), Err(Error::Unsupported(_))));
    // And a light nobody estimated is no light, not the last frame's.
    let mut light = crate::LightEstimate {
        primary_intensity: Vec3::X,
        ..Default::default()
    };
    session
        .light(crate::LightProbe::new(0), &mut light)
        .expect("no estimate is not an error");
    assert_eq!(light.primary_intensity, Vec3::ZERO);
}

#[test]
fn a_backend_without_hit_testing_says_so() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    assert!(matches!(
        session.hit_test_source(space),
        Err(Error::Unsupported(_))
    ));
    // And a hit test that never happened is no hits, not a failure.
    let mut hits = Vec::new();
    session
        .hits(crate::HitTestSource::new(0), space, &mut hits)
        .expect("an empty hit test is not an error");
    assert!(hits.is_empty());
}

#[test]
fn a_session_is_asked_for_in_a_mode_and_the_mock_remembers_it() {
    let session = MockBackend::default()
        .connect((), SessionMode::ImmersiveAr)
        .expect("the mock connects");
    assert_eq!(session.mode(), SessionMode::ImmersiveAr);
}

#[test]
fn a_controller_has_no_skeleton() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut sources = Vec::new();
    session.inputs(space, &mut sources).unwrap();
    assert!(!sources[0].hand, "a controller is not a hand");

    let mut hand = crate::Hand::default();
    session.hand(sources[0].id, space, &mut hand).unwrap();
    assert!(!hand.is_tracked());
    assert_eq!(hand.joint(crate::HandJoint::Wrist), None);
}

#[test]
fn the_set_of_inputs_changing_is_an_event_of_its_own() {
    let mut session = running();
    while session.poll().is_some() {}
    session.inputs_changed();
    assert_eq!(session.poll(), Some(Event::InputsChanged));
    assert_eq!(session.poll(), None);
}

#[test]
fn a_space_can_be_offset_from_another() {
    let mut session = running();
    let floor = session.space(SpaceKind::LocalFloor).expect("a floor");
    let table = session
        .offset_space(
            floor,
            Pose {
                position: Vec3::new(0.0, 0.75, 0.0),
                orientation: Quat::IDENTITY,
            },
        )
        .expect("an offset space");
    // Same kind, different handle: the offset is a new space that follows the one it was made from.
    assert_eq!(table.kind, floor.kind);
    assert_ne!(table.id(), floor.id());
}

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
fn a_mock_says_nothing_about_the_display_and_that_is_opaque() {
    // The default in the trait, and the right assumption: a session that does not say is one that fills
    // the display, which is what both ways of presenting into one do.
    let session = running();
    assert_eq!(session.blend(), crate::Blend::Opaque);
}

#[test]
fn a_space_without_a_floor_is_refused_rather_than_downgraded() {
    let mut session = running();
    assert!(matches!(
        session.space(SpaceKind::Viewer),
        Err(Error::NoSpace(SpaceKind::Viewer))
    ));
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
fn a_session_without_the_layer_feature_refuses_one() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).unwrap();
    let refused = session.layer(
        space,
        LayerShape::Quad {
            width: 1.0,
            height: 1.0,
        },
        Extent2d::new(256, 256),
    );
    // Refused, and not handed back a layer that nothing would ever place - which is what the capability bit is
    // for: an app that branches on `features` never sees this, and one that does not gets an error rather than a
    // silent nothing.
    assert!(matches!(refused, Err(Error::Unsupported(_))));
    assert!(!session.features().contains(Features::LAYER_QUAD));
    assert_eq!(session.layer_count(), 0);
}

/// The mock with the one capability it can have, polled to `Visible` like a real session.
fn with_layers() -> MockSession {
    let mut session = MockBackend {
        quad_layers: true,
        ..Default::default()
    }
    .connect((), SessionMode::ImmersiveVr)
    .unwrap();
    while let Some(event) = session.poll() {
        if let Event::VisibilityChanged(Visibility::Visible) = event {
            break;
        }
    }
    session
}

#[test]
fn a_layer_is_made_placed_drawn_into_and_released() {
    let mut session = with_layers();
    assert!(session.features().contains(Features::LAYER_QUAD));

    let space = session.space(SpaceKind::LocalFloor).unwrap();
    let shape = LayerShape::Quad {
        width: 1.2,
        height: 0.8,
    };
    let pixels = Extent2d::new(512, 384);
    let layer = session.layer(space, shape, pixels).unwrap();
    assert_eq!(session.layer_count(), 1);
    assert_eq!(session.layer_shape(layer), Some((shape, pixels)));
    assert_eq!(session.layer_space(layer), Some(space));

    let placed = Pose {
        position: Vec3::new(0.0, 1.5, -1.0),
        orientation: Quat::IDENTITY,
    };
    session.set_layer_pose(layer, placed).unwrap();
    assert_eq!(session.layer_pose(layer), Some(placed));

    // A frame hands out an image, and the next frame hands out a different one: a layer's picture belongs to the
    // frame, and a renderer that kept the first would draw into a picture the compositor has taken back.
    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    let (first, image) = session.layer_image(layer).unwrap();
    let first = *first;
    assert_eq!(image.meta.extent, pixels);
    assert_eq!(image.meta.layers, 1, "a quad is one picture for both eyes");
    assert!(image.is_whole(), "a layer gets a texture of its own");

    let mut frame = Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    let (second, _) = session.layer_image(layer).unwrap();
    assert_ne!(first, *second);

    session.release_layer(layer);
    assert_eq!(session.layer_count(), 0);
    assert!(session.layer_image(layer).is_none());
}

#[test]
fn a_shape_the_session_does_not_have_is_refused_even_with_layers() {
    let mut session = with_layers();
    let space = session.space(SpaceKind::LocalFloor).unwrap();
    let refused = session.layer(space, LayerShape::Cube, Extent2d::new(256, 256));
    // The bit is per shape on both platforms, so a session with a quad and no cube is the normal case rather
    // than an odd one.
    assert!(matches!(refused, Err(Error::Unsupported(_))));
    assert_eq!(session.layer_count(), 0);
}
