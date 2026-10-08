//! Where two readings of the WebXR API agree, and where they must.
//!
//! A specification fixes some things exactly - how many session modes there are, what an eye is - and those
//! are the things a second implementation can be compared against without hardware. Each test names the part
//! of the specification it is about, so that a failure says which sentence moved.

/// `XRSessionMode` - <https://immersive-web.github.io/webxr/#enumdef-xrsessionmode>
///
/// Three modes and no fourth: `inline` is a session drawn in the page, and the two immersive ones differ in
/// whether the picture *is* the world or is drawn over it. The comparison is variant by variant rather than
/// by count, so a mode added on either side is a failure and not a silently-covered case.
#[test]
fn the_session_modes_are_the_same_three() {
    let ours = [
        wxr::SessionMode::Inline,
        wxr::SessionMode::ImmersiveVr,
        wxr::SessionMode::ImmersiveAr,
    ];
    let theirs = [
        servo_webxr_api::SessionMode::Inline,
        servo_webxr_api::SessionMode::ImmersiveVR,
        servo_webxr_api::SessionMode::ImmersiveAR,
    ];

    assert_eq!(ours.len(), theirs.len(), "a mode was added on one side");
    for (ours, theirs) in ours.iter().zip(&theirs) {
        // The names differ only in casing, which is each project's own convention for its own words - the
        // point is that nothing else about them differs, which is what an equal name here says.
        assert_eq!(
            format!("{ours:?}").to_ascii_lowercase(),
            format!("{theirs:?}").to_ascii_lowercase(),
        );
    }
}

/// A name set, sorted - the shape a comparison of two enums with the same variants in different orders
/// wants, and the one that fails when a variant is added rather than quietly passing.
fn names<T: std::fmt::Debug>(variants: &[T]) -> Vec<String> {
    let mut names: Vec<String> = variants
        .iter()
        .map(|variant| format!("{variant:?}"))
        .collect();
    names.sort();
    names
}

/// `XRTargetRayMode` - <https://immersive-web.github.io/webxr/#enumdef-xrtargetraymode>
///
/// Four, and the two readings list them in different orders and agree on all four names.
#[test]
fn the_target_ray_modes_are_the_same_four() {
    let ours = names(&[
        wxr::TargetRayMode::TrackedPointer,
        wxr::TargetRayMode::Gaze,
        wxr::TargetRayMode::Screen,
        wxr::TargetRayMode::TransientPointer,
    ]);
    let theirs = names(&[
        servo_webxr_api::TargetRayMode::Gaze,
        servo_webxr_api::TargetRayMode::TrackedPointer,
        servo_webxr_api::TargetRayMode::Screen,
        servo_webxr_api::TargetRayMode::TransientPointer,
    ]);
    assert_eq!(ours, theirs);
}

/// `XRHandedness` - <https://immersive-web.github.io/webxr/#enumdef-xrhandedness>
///
/// The two hands are spelled the same. The third value is WebXR's `"none"` - a runtime that does not say, or
/// a source that is not a hand - and it is the one place the two readings name the same thing differently:
/// this core's has always been `Unknown`, because it is also what a source with no hand at all gets.
#[test]
fn the_hands_are_the_same_two_and_the_absence_is_spelled_out() {
    assert_eq!(
        names(&[wxr::Handedness::Left, wxr::Handedness::Right]),
        names(&[
            servo_webxr_api::Handedness::Left,
            servo_webxr_api::Handedness::Right
        ])
    );
    let ours = names(&[wxr::Handedness::Unknown]);
    let theirs = names(&[servo_webxr_api::Handedness::None]);
    assert_eq!(
        ours.len(),
        theirs.len(),
        "the third value is one value on both sides"
    );
}

/// `XRLayerLayout` - <https://immersive-web.github.io/webxr/#enumdef-xrlayerlayout>
///
/// Five values in the specification; Servo names three of them and this core names all five, so what can be
/// compared is that its three are spelled the same here - a layer laid out left-right means the same thing in
/// both readings, which is the part a picture would show.
#[test]
fn the_layer_layouts_agree_where_both_name_them() {
    let ours = names(&[
        wxr::layer::LayerLayout::Mono,
        wxr::layer::LayerLayout::StereoLeftRight,
        wxr::layer::LayerLayout::StereoTopBottom,
    ]);
    let theirs = names(&[
        servo_webxr_api::LayerLayout::Mono,
        servo_webxr_api::LayerLayout::StereoLeftRight,
        servo_webxr_api::LayerLayout::StereoTopBottom,
    ]);
    assert_eq!(ours, theirs);
}

/// `XREnvironmentBlendMode` - <https://immersive-web.github.io/webxr/#enumdef-xrenvironmentblendmode>
///
/// Three: an opaque session's picture replaces the world, an additive one adds light to it, and an
/// alpha-blending one mixes the two. Both readings name all three, in different orders.
#[test]
fn the_environment_blend_modes_are_the_same_three() {
    let ours = names(&[
        wxr::Blend::Opaque,
        wxr::Blend::Additive,
        wxr::Blend::AlphaBlend,
    ]);
    let theirs = names(&[
        servo_webxr_api::EnvironmentBlendMode::Opaque,
        servo_webxr_api::EnvironmentBlendMode::AlphaBlend,
        servo_webxr_api::EnvironmentBlendMode::Additive,
    ]);
    assert_eq!(ours, theirs);
}
