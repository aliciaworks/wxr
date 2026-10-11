//! ARKit's hand skeleton, read as the core's joints.
//!
//! **This file exists because a claim in this backend's own documentation turned out to be wrong.** For a
//! while `arkit`'s module comment said that ARKit's C API gives a hand anchor and its place in the world
//! and nothing else - "the skeleton is not in the C API, only the enumeration of joint *names* is, so there
//! are no joint transforms to read" - and that a game wanting fingers wanted the Swift `HandAnchor.Skeleton`
//! and therefore the app. That was a reading of the generated bindings, not of Apple's headers. The headers
//! say otherwise, in `skeleton_joint.h`:
//!
//! ```text
//! AR_EXTERN simd_float4x4 ar_skeleton_joint_get_anchor_from_joint_transform(ar_skeleton_joint_t joint);
//! ```
//!
//! The skeleton is in the C API and has been since visionOS 1.0. `ar_hand_anchor_get_hand_skeleton` hands
//! one over, `ar_hand_skeleton_get_joint_named` hands back a joint of it, and that getter returns a
//! `simd_float4x4` by value - which is the entire reason it looked absent: `objc2`'s translator cannot
//! express `simd` in a function, so the generated crate binds the joint's index, its parent and whether it
//! is tracked, and leaves the transform undeclared. A call that is *there* but *unbindable* reads the same
//! as a call that is not there, when all you have looked at is the bindings.
//!
//! So the transform crosses [`crate::sys`]'s shim like the other `simd` calls, and this is what is read:
//!
//! * the anchor's own place in the world, from `ar_hand_anchor_get_origin_from_anchor_transform`;
//! * each joint's place inside that anchor, from `ar_skeleton_joint_get_anchor_from_joint_transform`;
//! * whether the joint is tracked, from `ar_skeleton_joint_is_tracked`, which is what makes an untracked
//!   joint a [`None`] in the core's [`wxr::Hand`] rather than a joint at a stale pose.
//!
//! Both transforms are Apple's *rendering-corrected* form - each plain getter is documented as equal to its
//! `_with_correction` sibling asked for `ar_transform_correction_rendered` - so they compose. That is the
//! one property this file depends on and the easy one to get wrong: correct one and not the other and every
//! joint lands offset from the palm it belongs to, which draws as a hand that comes apart at the wrist.
//!
//! **Two of ARKit's joints have no home in the core, and that is deliberate.** ARKit names twenty-seven and
//! WebXR's hand has twenty-five: `forearm_wrist` and `forearm_arm` are the two over, and they are dropped
//! rather than folded into the wrist, because the core's [`wxr::HandJoint`] is WebXR's set and a forearm is
//! not a joint of a hand. The rest correspond one for one.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_hand_anchor_t, ar_hand_skeleton_joint_name_t, ar_hand_skeleton_t,
    ar_hand_tracking_provider_t, ar_skeleton_joint_t, ar_trackable_anchor_t,
};
use wxr::glam::Mat4;

use super::{ArKit, as_trackable_anchor};
use crate::sys;

/// ARKit's name for each of the core's joints, in the core's order.
///
/// ARKit happens to enumerate its first twenty-five joints in this same order, but the table is written out
/// rather than computed from that: a correspondence the code relies on and does not state is one a future
/// SDK renumbering would break silently, and a hand with the fingers shifted by one slot still draws.
const JOINTS: [ar_hand_skeleton_joint_name_t; wxr::HandJoint::COUNT] = [
    ar_hand_skeleton_joint_name_t::wrist,
    ar_hand_skeleton_joint_name_t::thumb_knuckle,
    ar_hand_skeleton_joint_name_t::thumb_intermediate_base,
    ar_hand_skeleton_joint_name_t::thumb_intermediate_tip,
    ar_hand_skeleton_joint_name_t::thumb_tip,
    ar_hand_skeleton_joint_name_t::index_finger_metacarpal,
    ar_hand_skeleton_joint_name_t::index_finger_knuckle,
    ar_hand_skeleton_joint_name_t::index_finger_intermediate_base,
    ar_hand_skeleton_joint_name_t::index_finger_intermediate_tip,
    ar_hand_skeleton_joint_name_t::index_finger_tip,
    ar_hand_skeleton_joint_name_t::middle_finger_metacarpal,
    ar_hand_skeleton_joint_name_t::middle_finger_knuckle,
    ar_hand_skeleton_joint_name_t::middle_finger_intermediate_base,
    ar_hand_skeleton_joint_name_t::middle_finger_intermediate_tip,
    ar_hand_skeleton_joint_name_t::middle_finger_tip,
    ar_hand_skeleton_joint_name_t::ring_finger_metacarpal,
    ar_hand_skeleton_joint_name_t::ring_finger_knuckle,
    ar_hand_skeleton_joint_name_t::ring_finger_intermediate_base,
    ar_hand_skeleton_joint_name_t::ring_finger_intermediate_tip,
    ar_hand_skeleton_joint_name_t::ring_finger_tip,
    ar_hand_skeleton_joint_name_t::little_finger_metacarpal,
    ar_hand_skeleton_joint_name_t::little_finger_knuckle,
    ar_hand_skeleton_joint_name_t::little_finger_intermediate_base,
    ar_hand_skeleton_joint_name_t::little_finger_intermediate_tip,
    ar_hand_skeleton_joint_name_t::little_finger_tip,
];

/// How thick a finger is where ARKit does not say, in metres.
///
/// ARKit reports a joint's transform and whether it is tracked, and that is all: no radius, unlike OpenXR
/// and WebXR, which both report one. The core's `Joint::radius` is what turns a hand drawn from joints into
/// something with fingers rather than a stick figure, so a joint with no radius is a joint that draws as a
/// point - which makes a constant the better answer than a zero, and 8 mm is an average finger.
const RADIUS: f32 = 0.008;

/// One hand's skeleton, in `origin_from_hand`'s space.
///
/// `origin_from_hand` is the whole way from wherever the caller asked to the hand anchor's own frame - the
/// anchor's place in the world composed in front of the reference space's origin, exactly as `inputs` brings
/// a palm across. Each joint is then placed by the anchor-to-joint transform alone.
///
/// An empty [`wxr::Hand`] is the answer to a hand with no skeleton: the anchor is live but ARKit is not
/// tracking it, which is a hand out of view and not an error.
///
/// # Safety
///
/// `anchor` must be a live hand anchor from the generated crate.
pub(super) unsafe fn read(anchor: &ar_hand_anchor_t, origin_from_hand: Mat4) -> wxr::Hand {
    let mut hand = wxr::Hand::default();
    // SAFETY: the anchor is live; the skeleton is the anchor's own and is retained by the `Retained` below.
    let Some(skeleton) = (unsafe { ar_hand_anchor_t::hand_skeleton(anchor) }) else {
        return hand;
    };
    for joint in wxr::HandJoint::ALL {
        // SAFETY: the skeleton is live and the name is one of ARKit's own - the header declares this getter
        // as returning a joint and not a nullable one, so a name it knows cannot come back empty.
        let located = unsafe { ar_hand_skeleton_t::joint_named(&skeleton, JOINTS[joint.index()]) };
        // A joint ARKit is not tracking this frame, which is what an empty slot in the core's `Hand` means.
        // SAFETY: the joint is live.
        if !unsafe { ar_skeleton_joint_t::is_tracked(&located) } {
            continue;
        }
        // SAFETY: the joint is live, and reading a transform from one does not consume it.
        let joint_from_anchor = unsafe {
            sys::ar_skeleton_joint_get_anchor_from_joint_transform(
                Retained::as_ptr(&located).cast::<c_void>(),
            )
        };
        let pose = wxr_render::pose_from_transform(
            origin_from_hand * Mat4::from_cols_array(&joint_from_anchor.0),
        );
        hand.joints_mut()[joint.index()] = Some(wxr::Joint {
            pose,
            radius: RADIUS,
        });
    }
    hand
}

impl ArKit {
    /// Whether a hand provider came up, which is what `features` asks.
    ///
    /// The provider and not the skeleton: a device that can ask about hands is a device with
    /// [`wxr::Features::HAND_TRACKING`], and whether any particular joint is being tracked this frame is the
    /// per-joint question [`ArKit::hand`] answers.
    pub fn has_hands(&self) -> bool {
        self.hands.is_some()
    }

    /// Where the hands are, as `(handedness, place in the world, tracked)`.
    ///
    /// Empty when there is no hand provider, when it has nothing yet, or when the query failed - all three
    /// of which are a frame with no hands rather than a frame to stop over. Handedness comes from which
    /// anchor it is, because the call hands over a left one and a right one and says so in its arguments.
    pub fn hands(&self) -> Vec<(wxr::Handedness, Mat4, bool)> {
        let Some(hands) = &self.hands else {
            return Vec::new();
        };
        // SAFETY: the provider and both anchors are live, and the anchors are ours to fill.
        let latest = unsafe {
            ar_hand_tracking_provider_t::latest_anchors(&hands.provider, &hands.left, &hands.right)
        };
        if !latest {
            return Vec::new();
        }
        [
            (wxr::Handedness::Left, &hands.left),
            (wxr::Handedness::Right, &hands.right),
        ]
        .into_iter()
        .map(|(handedness, anchor)| {
            // SAFETY: both casts are the subtype ones above; the anchor is live and the transform is the
            // hand's place in the world.
            let transform = unsafe {
                sys::ar_hand_anchor_get_origin_from_anchor_transform(
                    Retained::as_ptr(anchor).cast(),
                )
            };
            let tracked = unsafe { ar_trackable_anchor_t::is_tracked(as_trackable_anchor(anchor)) };
            (handedness, Mat4::from_cols_array(&transform.0), tracked)
        })
        .collect()
    }

    /// One hand's skeleton, in `origin`'s space - the same transform `inputs` brings a palm across, so a
    /// joint and the palm it belongs to land in the same place.
    ///
    /// `handedness` is which hand, and an [`wxr::Handedness::Unknown`] is a source that is not a hand at
    /// all: no answer rather than an arbitrary one, because guessing which of two hands a caller meant
    /// would put the wrong fingers in it.
    pub fn hand(&self, handedness: wxr::Handedness, origin: Mat4) -> Option<(bool, wxr::Hand)> {
        let hands = self.hands.as_ref()?;
        // SAFETY: the provider and both anchors are live, and the anchors are ours to fill. This is the
        // same query `hands` makes, repeated rather than cached: the anchors ARKit fills are the ones the
        // provider's *latest* update wrote, and a frame that asked for inputs and then asked for a
        // skeleton wants the skeleton of that same update.
        let latest = unsafe {
            ar_hand_tracking_provider_t::latest_anchors(&hands.provider, &hands.left, &hands.right)
        };
        if !latest {
            return None;
        }
        let anchor = match handedness {
            wxr::Handedness::Left => &hands.left,
            wxr::Handedness::Right => &hands.right,
            wxr::Handedness::Unknown => return None,
        };
        let transform = unsafe {
            sys::ar_hand_anchor_get_origin_from_anchor_transform(Retained::as_ptr(anchor).cast())
        };
        let tracked = unsafe { ar_trackable_anchor_t::is_tracked(as_trackable_anchor(anchor)) };
        // SAFETY: the anchor is the live one the provider just filled.
        let hand = unsafe { read(anchor, origin * Mat4::from_cols_array(&transform.0)) };
        Some((tracked, hand))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same twenty-five joints, named by both vocabularies at once.
    ///
    /// Four of the pairs are the reason this is written out rather than derived: ARKit's `knuckle` is
    /// WebXR's `phalanx proximal`, its `intermediate_base` is WebXR's `phalanx intermediate`, its
    /// `intermediate_tip` is WebXR's `phalanx distal`, and its `little finger` is WebXR's `pinky`. None of
    /// those four is a translation anything can do by string, and a hand with the middle two swapped still
    /// draws - it draws a finger with a joint in the wrong place along it.
    const CORRESPONDENCE: [(ar_hand_skeleton_joint_name_t, wxr::HandJoint); wxr::HandJoint::COUNT] = [
        (ar_hand_skeleton_joint_name_t::wrist, wxr::HandJoint::Wrist),
        (
            ar_hand_skeleton_joint_name_t::thumb_knuckle,
            wxr::HandJoint::ThumbMetacarpal,
        ),
        (
            ar_hand_skeleton_joint_name_t::thumb_intermediate_base,
            wxr::HandJoint::ThumbPhalanxProximal,
        ),
        (
            ar_hand_skeleton_joint_name_t::thumb_intermediate_tip,
            wxr::HandJoint::ThumbPhalanxDistal,
        ),
        (
            ar_hand_skeleton_joint_name_t::thumb_tip,
            wxr::HandJoint::ThumbTip,
        ),
        (
            ar_hand_skeleton_joint_name_t::index_finger_metacarpal,
            wxr::HandJoint::IndexFingerMetacarpal,
        ),
        (
            ar_hand_skeleton_joint_name_t::index_finger_knuckle,
            wxr::HandJoint::IndexFingerPhalanxProximal,
        ),
        (
            ar_hand_skeleton_joint_name_t::index_finger_intermediate_base,
            wxr::HandJoint::IndexFingerPhalanxIntermediate,
        ),
        (
            ar_hand_skeleton_joint_name_t::index_finger_intermediate_tip,
            wxr::HandJoint::IndexFingerPhalanxDistal,
        ),
        (
            ar_hand_skeleton_joint_name_t::index_finger_tip,
            wxr::HandJoint::IndexFingerTip,
        ),
        (
            ar_hand_skeleton_joint_name_t::middle_finger_metacarpal,
            wxr::HandJoint::MiddleFingerMetacarpal,
        ),
        (
            ar_hand_skeleton_joint_name_t::middle_finger_knuckle,
            wxr::HandJoint::MiddleFingerPhalanxProximal,
        ),
        (
            ar_hand_skeleton_joint_name_t::middle_finger_intermediate_base,
            wxr::HandJoint::MiddleFingerPhalanxIntermediate,
        ),
        (
            ar_hand_skeleton_joint_name_t::middle_finger_intermediate_tip,
            wxr::HandJoint::MiddleFingerPhalanxDistal,
        ),
        (
            ar_hand_skeleton_joint_name_t::middle_finger_tip,
            wxr::HandJoint::MiddleFingerTip,
        ),
        (
            ar_hand_skeleton_joint_name_t::ring_finger_metacarpal,
            wxr::HandJoint::RingFingerMetacarpal,
        ),
        (
            ar_hand_skeleton_joint_name_t::ring_finger_knuckle,
            wxr::HandJoint::RingFingerPhalanxProximal,
        ),
        (
            ar_hand_skeleton_joint_name_t::ring_finger_intermediate_base,
            wxr::HandJoint::RingFingerPhalanxIntermediate,
        ),
        (
            ar_hand_skeleton_joint_name_t::ring_finger_intermediate_tip,
            wxr::HandJoint::RingFingerPhalanxDistal,
        ),
        (
            ar_hand_skeleton_joint_name_t::ring_finger_tip,
            wxr::HandJoint::RingFingerTip,
        ),
        (
            ar_hand_skeleton_joint_name_t::little_finger_metacarpal,
            wxr::HandJoint::PinkyFingerMetacarpal,
        ),
        (
            ar_hand_skeleton_joint_name_t::little_finger_knuckle,
            wxr::HandJoint::PinkyFingerPhalanxProximal,
        ),
        (
            ar_hand_skeleton_joint_name_t::little_finger_intermediate_base,
            wxr::HandJoint::PinkyFingerPhalanxIntermediate,
        ),
        (
            ar_hand_skeleton_joint_name_t::little_finger_intermediate_tip,
            wxr::HandJoint::PinkyFingerPhalanxDistal,
        ),
        (
            ar_hand_skeleton_joint_name_t::little_finger_tip,
            wxr::HandJoint::PinkyFingerTip,
        ),
    ];

    #[test]
    fn the_table_reads_the_same_joints_the_core_names() {
        for (index, (arkit, webxr)) in CORRESPONDENCE.into_iter().enumerate() {
            assert_eq!(JOINTS[index], arkit, "the table's joint {index}");
            assert_eq!(
                wxr::HandJoint::ALL[index],
                webxr,
                "the core's joint {index}"
            );
        }
    }

    /// A canary rather than a correctness claim: `read` looks each joint up *by name*, so a SDK that
    /// renumbers its joints would still read the right one. What this is for is the other line in `JOINTS`
    /// comment - that ARKit happens to enumerate its first twenty-five in this same order - and for making
    /// the insertion of a twenty-sixth joint *somewhere* a failure somebody looks at rather than a fact
    /// nobody notices.
    #[test]
    fn ar_kit_numbers_the_joints_it_shares_in_the_order_the_table_reads() {
        for (index, joint) in JOINTS.into_iter().enumerate() {
            assert_eq!(joint.0, index as u64, "joint {index} is numbered elsewhere");
        }
        // The two over, which are the two the core has no name for and are dropped rather than folded in.
        assert_eq!(
            ar_hand_skeleton_joint_name_t::forearm_wrist.0,
            JOINTS.len() as u64
        );
        assert_eq!(
            ar_hand_skeleton_joint_name_t::forearm_arm.0,
            JOINTS.len() as u64 + 1
        );
    }

    #[test]
    fn a_hand_has_as_many_joints_as_the_table_has_names() {
        assert_eq!(wxr::HandJoint::COUNT, JOINTS.len());
    }

    /// ARKit reports no radius, so this is the whole of what makes a drawn hand a hand: a joint with a
    /// zero radius is a point, and a hand drawn from points is a stick figure. The value is read back off
    /// a joint rather than asserted as a constant so that the shape of the answer is what is checked.
    #[test]
    fn a_joint_has_a_thickness_even_though_ar_kit_gives_none() {
        let joint = wxr::Joint {
            pose: wxr::Pose::IDENTITY,
            radius: RADIUS,
        };
        assert!(joint.radius > 0.0);
        // An average finger is not a bowling ball: it is meant to be a joint's thickness in metres, so
        // anything past a centimetre is a mistake rather than a judgement call.
        assert!(joint.radius < 0.01);
    }
}
