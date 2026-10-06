//! Platform values in the core's terms.
//!
//! Everything here is one direction or the other between what `web-sys` reports and what the core says, kept
//! together so the mappings can be read next to each other.

use wasm_bindgen::prelude::*;
use web_sys::{
    XrHandJoint, XrReferenceSpace, XrReferenceSpaceType, XrRigidTransform, XrSessionMode, XrView,
};

use wxr::glam::{Quat, Vec3};

/// A browser's visibility state in the core's terms, which is a translation and not a mapping.
///
/// It is the same vocabulary: WebXR is where the core's [`wxr::Visibility`] came from, three rungs and all.
/// Nothing is folded and nothing is invented.
pub(crate) fn visibility(state: web_sys::XrVisibilityState) -> wxr::Visibility {
    match state {
        web_sys::XrVisibilityState::Visible => wxr::Visibility::Visible,
        web_sys::XrVisibilityState::VisibleBlurred => wxr::Visibility::VisibleBlurred,
        _ => wxr::Visibility::Hidden,
    }
}

/// What scale the runtime suggests for a view, which WebXR reports as a nullable number.
pub(crate) fn recommended_scale(view: &XrView) -> Option<f32> {
    js_sys::Reflect::get(
        view.unchecked_ref::<JsValue>(),
        &JsValue::from_str("recommendedViewportScale"),
    )
    .ok()
    .and_then(|value| value.as_f64())
    .map(|value| value as f32)
}

/// Which browser session mode a core one is, which is WebXR's own three values.
pub(crate) fn session_mode(mode: wxr::SessionMode) -> XrSessionMode {
    match mode {
        wxr::SessionMode::Inline => XrSessionMode::Inline,
        wxr::SessionMode::ImmersiveVr => XrSessionMode::ImmersiveVr,
        wxr::SessionMode::ImmersiveAr => XrSessionMode::ImmersiveAr,
    }
}

/// Which WebXR reference space a core one is.
pub(crate) fn reference_space_type(kind: wxr::SpaceKind) -> XrReferenceSpaceType {
    match kind {
        wxr::SpaceKind::Viewer => XrReferenceSpaceType::Viewer,
        wxr::SpaceKind::Local => XrReferenceSpaceType::Local,
        wxr::SpaceKind::LocalFloor => XrReferenceSpaceType::LocalFloor,
        wxr::SpaceKind::BoundedFloor => XrReferenceSpaceType::BoundedFloor,
        // WebXR's unbounded space is a first-class one, which is the other way round from OpenXR.
        wxr::SpaceKind::Unbounded => XrReferenceSpaceType::Unbounded,
    }
}

/// A space at `offset` inside `base`, which is WebXR's `getOffsetReferenceSpace`.
pub(crate) fn offset_reference_space(
    base: &XrReferenceSpace,
    offset: wxr::Pose,
) -> Result<XrReferenceSpace, wxr::Error> {
    let transform = rigid(offset)?;
    Ok(base.get_offset_reference_space(&transform))
}

/// The core's pose as the browser's rigid transform.
///
/// A position is a point and a quaternion one with four coordinates, which is the shape the constructor asks
/// for - the constructor, because an `XRRigidTransform` has no setters worth using.
pub(crate) fn rigid(pose: wxr::Pose) -> Result<XrRigidTransform, wxr::Error> {
    let position = web_sys::DomPointInit::new();
    position.set_x(pose.position.x as f64);
    position.set_y(pose.position.y as f64);
    position.set_z(pose.position.z as f64);
    let orientation = web_sys::DomPointInit::new();
    orientation.set_x(pose.orientation.x as f64);
    orientation.set_y(pose.orientation.y as f64);
    orientation.set_z(pose.orientation.z as f64);
    orientation.set_w(pose.orientation.w as f64);
    XrRigidTransform::new_with_position_and_orientation(&position, &orientation)
        .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))
}

/// Which browser joint a core one is.
///
/// A one-to-one match, because it is WebXR's own list: the core took the names from the specification rather
/// than inventing a vocabulary of its own.
pub(crate) fn hand_joint(joint: wxr::HandJoint) -> XrHandJoint {
    match joint {
        wxr::HandJoint::Wrist => XrHandJoint::Wrist,
        wxr::HandJoint::ThumbMetacarpal => XrHandJoint::ThumbMetacarpal,
        wxr::HandJoint::ThumbPhalanxProximal => XrHandJoint::ThumbPhalanxProximal,
        wxr::HandJoint::ThumbPhalanxDistal => XrHandJoint::ThumbPhalanxDistal,
        wxr::HandJoint::ThumbTip => XrHandJoint::ThumbTip,
        wxr::HandJoint::IndexFingerMetacarpal => XrHandJoint::IndexFingerMetacarpal,
        wxr::HandJoint::IndexFingerPhalanxProximal => XrHandJoint::IndexFingerPhalanxProximal,
        wxr::HandJoint::IndexFingerPhalanxIntermediate => {
            XrHandJoint::IndexFingerPhalanxIntermediate
        }
        wxr::HandJoint::IndexFingerPhalanxDistal => XrHandJoint::IndexFingerPhalanxDistal,
        wxr::HandJoint::IndexFingerTip => XrHandJoint::IndexFingerTip,
        wxr::HandJoint::MiddleFingerMetacarpal => XrHandJoint::MiddleFingerMetacarpal,
        wxr::HandJoint::MiddleFingerPhalanxProximal => XrHandJoint::MiddleFingerPhalanxProximal,
        wxr::HandJoint::MiddleFingerPhalanxIntermediate => {
            XrHandJoint::MiddleFingerPhalanxIntermediate
        }
        wxr::HandJoint::MiddleFingerPhalanxDistal => XrHandJoint::MiddleFingerPhalanxDistal,
        wxr::HandJoint::MiddleFingerTip => XrHandJoint::MiddleFingerTip,
        wxr::HandJoint::RingFingerMetacarpal => XrHandJoint::RingFingerMetacarpal,
        wxr::HandJoint::RingFingerPhalanxProximal => XrHandJoint::RingFingerPhalanxProximal,
        wxr::HandJoint::RingFingerPhalanxIntermediate => XrHandJoint::RingFingerPhalanxIntermediate,
        wxr::HandJoint::RingFingerPhalanxDistal => XrHandJoint::RingFingerPhalanxDistal,
        wxr::HandJoint::RingFingerTip => XrHandJoint::RingFingerTip,
        wxr::HandJoint::PinkyFingerMetacarpal => XrHandJoint::PinkyFingerMetacarpal,
        wxr::HandJoint::PinkyFingerPhalanxProximal => XrHandJoint::PinkyFingerPhalanxProximal,
        wxr::HandJoint::PinkyFingerPhalanxIntermediate => {
            XrHandJoint::PinkyFingerPhalanxIntermediate
        }
        wxr::HandJoint::PinkyFingerPhalanxDistal => XrHandJoint::PinkyFingerPhalanxDistal,
        wxr::HandJoint::PinkyFingerTip => XrHandJoint::PinkyFingerTip,
    }
}

/// A transform in the core's terms.
pub(crate) fn transform(transform: web_sys::XrRigidTransform) -> wxr::Pose {
    let position = transform.position();
    let orientation = transform.orientation();
    // The browser reports these as doubles, and a pose is `f32` everywhere else.
    wxr::Pose {
        position: Vec3::new(
            position.x() as f32,
            position.y() as f32,
            position.z() as f32,
        ),
        orientation: Quat::from_xyzw(
            orientation.x() as f32,
            orientation.y() as f32,
            orientation.z() as f32,
            orientation.w() as f32,
        ),
    }
}

/// A field of view, derived from the projection matrix.
///
/// WebXR does not report angles: it reports the matrix the eyes are projected with, and the four openings
/// have to be read back out of it. The matrix is column-major and this is the standard inverse - the
/// horizontal scale is the first element, the vertical the sixth, and the two offsets are what make the
/// projection asymmetric.
pub(crate) fn field_of_view(projection: &[f32]) -> wxr::FieldOfView {
    let at = |index: usize| projection[index];
    let (x_scale, y_scale) = (at(0), at(5));
    let (x_offset, y_offset) = (at(8), at(9));
    if x_scale == 0.0 || y_scale == 0.0 {
        return wxr::FieldOfView::symmetric(0.0, 0.0);
    }
    wxr::FieldOfView {
        up: ((y_offset + 1.0) / y_scale).atan(),
        down: (-((y_offset - 1.0) / y_scale)).atan(),
        left: (-((x_offset - 1.0) / x_scale)).atan(),
        right: ((x_offset + 1.0) / x_scale).atan(),
    }
}
