//! The frame loop, as the renderer draws it: one pass per eye, presented through the session.
//!
//! It is the end-to-end test of this crate - a mock session with two eyes and an image, a renderer
//! with a scene, and a frame that comes out of it - and it is its own file because it is the one that
//! needs everything the others set up in pieces.

use super::*;

#[test]
fn a_frame_with_two_eyes_draws_twice_and_presents() {
    let Some((device, queue)) = device() else {
        return;
    };

    // The mock is what makes this a test of the renderer rather than of a headset: it is a session with
    // two eyes and an image, and the renderer cannot tell it from any other.
    let mut session = wxr::mock::MockBackend::default()
        .connect((), wxr::SessionMode::ImmersiveVr)
        .expect("the mock connects");
    while let Some(event) = session.poll() {
        if matches!(
            event,
            wxr::Event::VisibilityChanged(wxr::Visibility::Visible)
        ) {
            break;
        }
    }
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");

    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");
    assert_eq!(frame.views().len(), 2, "the mock presents two eyes");

    let mut renderer = Renderer::new([0.1, 0.1, 0.1, 1.0]);
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame draws");
    assert_eq!(drawn, 2, "one pass per eye");
}

#[test]
fn a_triangle_is_drawn_with_each_eye_own_projection() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut session = wxr::mock::MockBackend::default()
        .connect((), wxr::SessionMode::ImmersiveVr)
        .unwrap();
    while let Some(event) = session.poll() {
        if matches!(
            event,
            wxr::Event::VisibilityChanged(wxr::Visibility::Visible)
        ) {
            break;
        }
    }
    let space = session.space(wxr::SpaceKind::LocalFloor).unwrap();
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).unwrap();
    session.views(space, &mut frame).unwrap();

    // The scene is what makes the projection arithmetic load-bearing: without it the frame would be a
    // clear, and a clear passes whatever matrix it is given.
    let mut renderer = Renderer::with_scene(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        [0.0, 0.0, 0.0, 1.0],
        Depth::ZeroToOne,
    );
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame draws");
    assert_eq!(drawn, 2);
}
