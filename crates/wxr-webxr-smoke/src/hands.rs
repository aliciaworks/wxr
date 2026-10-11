//! What a hand says about itself, asserted on a value rather than on a count.
//!
//! A skeleton is twenty-five joints, and twenty-five is the number a hand reports whether it read them the
//! batched way or the one-at-a-time way - so a count would pass on either path and say nothing about whether
//! the batched one got the *numbers* right. `fillPoses` hands over matrices and no transform objects, and a
//! matrix read the wrong way round still fills in twenty-five joints; it just puts all of them somewhere else.
//!
//! So the shape is what is checked: two distances that the stub fixes and no reading of a matrix can keep.

use super::*;

/// Where the wrist is, and how far the index tip is from it.
///
/// Both come from `webxr-stub.js`, which is why they are numbers in this file: the stub puts its hands at a
/// wrist height of 1.55 m and its index tip ten centimetres along, a centimetre up and two forward. A
/// transposed matrix puts the wrist at the origin and a scaled one grows the finger, and neither answer keeps
/// those two numbers.
pub(super) fn check_hand(hand: &wxr::Hand, handedness: wxr::Handedness) -> Result<(), String> {
    let tracked = wxr::HandJoint::ALL
        .iter()
        .filter(|joint| hand.joint(**joint).is_some())
        .count();
    let Some(wrist) = hand.joint(wxr::HandJoint::Wrist) else {
        return Err(format!(
            "a {handedness:?} hand with {tracked} of {} joints has no wrist",
            wxr::HandJoint::COUNT
        ));
    };
    let up = wrist.pose.position.y;
    if (up - 1.55).abs() > 0.01 {
        return Err(format!(
            "the wrist is {up:.3} m up and the stub puts it at 1.55"
        ));
    }
    let Some(tip) = hand.joint(wxr::HandJoint::IndexFingerTip) else {
        return Err(format!(
            "a {handedness:?} hand with {tracked} of {} joints has no index tip",
            wxr::HandJoint::COUNT
        ));
    };
    let reach = (tip.pose.position - wrist.pose.position).length();
    if (reach - 0.1025).abs() > 0.002 {
        return Err(format!(
            "the wrist is {reach:.4} m from the index tip and the stub makes it 0.1025"
        ));
    }
    say(&format!(
        "  {handedness:?} wrist at {up:.3} m up, {reach:.4} m from the index tip, radius {:.4}",
        wrist.radius,
    ));
    Ok(())
}
