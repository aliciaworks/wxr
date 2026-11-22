//! Layers on the mock: what a session without them refuses, and what one with a quad keeps.

use super::*;

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
        LayerOptions::default(),
    );
    // Refused rather than handed a layer nothing would place: an app branching on `features` never gets here.
    assert!(matches!(refused, Err(Error::Unsupported(_))));
    assert!(!session.features().contains(Features::LAYER_QUAD));
    assert_eq!(session.layer_count(), 0);
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
    // An opaque picture, so the test sees the app's answer arrive rather than the default the mock would keep
    // either way.
    let options = LayerOptions {
        blend_texture_source_alpha: false,
    };
    let layer = session.layer(space, shape, pixels, options).unwrap();
    assert_eq!(session.layer_count(), 1);
    assert_eq!(session.layer_shape(layer), Some((shape, pixels)));
    assert_eq!(session.layer_space(layer), Some(space));
    assert_eq!(session.layer_options(layer), Some(options));

    let placed = Pose {
        position: Vec3::new(0.0, 1.5, -1.0),
        orientation: Quat::IDENTITY,
    };
    session.set_layer_pose(layer, placed).unwrap();
    assert_eq!(session.layer_pose(layer), Some(placed));

    // A layer's picture belongs to the frame: a renderer that kept the first would draw into a taken picture.
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
    let refused = session.layer(
        space,
        LayerShape::Cube,
        Extent2d::new(256, 256),
        LayerOptions::default(),
    );
    // The bit is per shape on both platforms, so a session with a quad and no cube is the normal case.
    assert!(matches!(refused, Err(Error::Unsupported(_))));
    assert_eq!(session.layer_count(), 0);
}
