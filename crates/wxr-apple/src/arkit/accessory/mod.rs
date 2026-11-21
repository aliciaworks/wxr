//! Accessories: where a controller or a stylus is, which is the half GameController does not have.
//!
//! A device reports buttons - and, on a stylus, pressure - and an actuator; ARKit's accessory tracking is
//! what says where it is in the room.
//! The two halves are joined by a `GCDevice`: `ar_accessory_load_from_device_f` asks ARKit to make an
//! accessory of one, and every accessory anchor ARKit then reports names the device it came from, which is
//! the same object GameController hands out - so a pointer is the one name the two frameworks share, and
//! [`AccessoryPose::device`] is that pointer.
//!
//! **The provider is made after the accessories it tracks.** Its configuration names them at creation, and a
//! load answers on a queue of ARKit's own, so a session that started a provider with an empty set would be a
//! session tracking nothing. [`Accessories::provider`] is therefore called once a frame: it takes whatever
//! has loaded, starts the provider the first time there is something for it, and on visionOS 27 adds later
//! arrivals with `ar_accessory_tracking_provider_update_accessories_f`. On visionOS 26 an accessory that
//! arrives after the provider was made is a pad with buttons and no place until the session is made again,
//! which is the ordering the header gives rather than a choice this module made.
//!
//! **A loaded accessory crosses threads as an address.** The completion runs on ARKit's queue and cannot
//! hand a `Retained<ar_accessory_t>` to a frame loop without claiming the object is `Send`, which the header
//! does not. So the callback takes a `+1` and stores its address, exactly like the world-anchor listing
//! beside this, and only this file turns one back into a [`Retained`] - on the thread that calls
//! [`Accessories::provider`].
//!
//! **Three transforms, one accessory.** The anchor's own transform is where the accessory is tracked, and
//! Apple's own forum says it is arbitrary: it need not be a place on the hardware a caller would point from.
//! What is not arbitrary is the named locations - a pad labels `aim`, `grip` and a grip surface, and a
//! stylus labels only `aim` - so the core's aim comes from the aim location and its grip from the grip one,
//! both times the anchor's transform with the location's folded in. An accessory that names neither keeps
//! the anchor's transform for both, which is what the core did before the locations were read at all.
//!
//! Every one of them is asked for *uncorrected*: ARKit's plain call is the rendering correction, moved to
//! land over the physical accessory in passthrough, and a pose an app points a ray with is where the
//! accessory actually is. Read the aim and grip as a stylus tip and a palm, and the difference is the
//! difference between a crosshair that lands on the thing and one that lands beside it.
//!
//! **And how fast it is going.** The anchor also carries ARKit's estimate of its velocity, in metres per
//! second along ARKit's axes, and of its angular velocity, in radians per second about its own; both go into
//! [`wxr::InputSource::motion`]. A velocity is a direction per second, so it turns with the caller's space
//! and is not moved by it, and a speed about the accessory's own axes reads the same however that space is
//! turned.
//!
//! **And where it will be when the frame is drawn.** A latest anchor is where the accessory was when ARKit
//! last updated it, which is a frame or more before the picture that uses it - so a frame that knows its
//! trackable anchor time asks ARKit to predict each tracked anchor to it, and reads the place from the
//! predicted one. `cp_frame_timing_get_trackable_anchor_time` is the compositor's own answer to which time
//! that is, and the header names exactly this use for it. What prediction does not change is identity: the
//! accessory, the hand and the tracking state are the latest anchor's, and a prediction ARKit refuses leaves
//! the latest anchor as the answer rather than no pose at all.

mod provider;

use objc2::runtime::ProtocolObject;
use objc2_ar_kit::{ar_accessory_anchor_tracking_state_t, ar_accessory_chirality_t};
use objc2_game_controller::GCDevice;
use wxr::glam::Mat4;

pub(crate) use provider::Accessories;

use super::ArKit;

/// Where one accessory is, how it is moving, and which controller it is.
pub(crate) struct AccessoryPose {
    /// The `GCDevice` this anchor was loaded from, as the address GameController knows it by. The pointer is
    /// the only name the two frameworks share, and comparing it is how a controller finds its own place.
    pub(crate) device: usize,
    /// Which hand the accessory belongs to: the one holding it now, or for one set down the one it is
    /// designed for. `Unknown` only when ARKit answers neither.
    pub(crate) handedness: wxr::Handedness,
    /// Whether ARKit is tracking it right now: an accessory on a table is still placed, one the runtime has
    /// lost is not.
    pub(crate) tracked: bool,
    /// Whether the *position* is a guess because only the orientation is tracked, which is the core's
    /// `emulated_position` and a different question from `tracked`.
    pub(crate) emulated_position: bool,
    /// Whether a hand is holding it: a pad on a table is tracked and not held, which is what
    /// `ar_accessory_anchor_is_held` answers.
    pub(crate) held: bool,
    /// The grip's place and orientation, in ARKit's world: the named grip location when there is one, and
    /// the anchor's own transform when the accessory names none.
    pub(crate) grip: Mat4,
    /// The aim's place and orientation, in ARKit's world, from the named aim location or the anchor.
    pub(crate) aim: Mat4,
    /// Metres per second along ARKit's axes, estimated by the runtime rather than differenced here.
    pub(crate) linear_velocity: wxr::glam::Vec3,
    /// Radians per second about the accessory's own axes.
    pub(crate) angular_velocity: wxr::glam::Vec3,
}

/// Whether the position is a guess, from ARKit's richer answer about what it tracks: orientation alone is
/// the three-degrees-of-freedom case the core's flag is about, and a low accuracy position is still measured.
fn emulated_position_of(state: ar_accessory_anchor_tracking_state_t) -> bool {
    state == ar_accessory_anchor_tracking_state_t::orientation_tracked
}

/// Which hand an accessory belongs to, from the two answers ARKit gives about it.
///
/// `held` is the hand holding it now and `inherent` is the hand the hardware is made for. The first is what
/// the wearer is doing and the second is what the accessory is, and one on a table has only the second - so
/// the held answer wins when there is one, and the inherent one is the fallback. Apple's own enum says the
/// two are one question: `left` is "left-handed accessory, or non-handed accessory held in left hand".
fn handedness_of(
    held: ar_accessory_chirality_t,
    inherent: ar_accessory_chirality_t,
) -> wxr::Handedness {
    match (chirality_of(held), chirality_of(inherent)) {
        (wxr::Handedness::Unknown, designed) => designed,
        (held, _) => held,
    }
}

/// One of ARKit's chirality answers as the core's word for it.
fn chirality_of(answer: ar_accessory_chirality_t) -> wxr::Handedness {
    match answer {
        ar_accessory_chirality_t::left => wxr::Handedness::Left,
        ar_accessory_chirality_t::right => wxr::Handedness::Right,
        // `unspecified`, and anything ARKit adds later: a non-handed accessory in nobody's hand.
        _ => wxr::Handedness::Unknown,
    }
}

impl ArKit {
    /// Ask ARKit to load the accessory behind a controller, once per `GCDevice`.
    ///
    /// A session without accessory tracking - an older system, or a device without the provider - does
    /// nothing here, and its controllers stay button-only sources.
    pub(crate) fn load_accessory(&self, device: &ProtocolObject<dyn GCDevice>) {
        if let Some(accessories) = &self.accessories {
            accessories.load(device);
        }
    }

    /// Where the accessories are, after the provider has been given whatever loaded since the last call.
    ///
    /// `&mut self` because the provider is made here rather than at startup: the set it tracks is not known
    /// until ARKit has answered a load. `at` predicts each tracked anchor to that Mach absolute time; `None`
    /// is a caller outside a frame, which gets the latest anchors.
    pub(crate) fn accessory_poses(&mut self, at: Option<f64>) -> Vec<AccessoryPose> {
        let Some(accessories) = &mut self.accessories else {
            return Vec::new();
        };
        accessories.provider(&self._session, &self._providers);
        accessories.poses(at)
    }
}

/// Where a controller is and how it moves, in the space being reported in.
pub(crate) struct Place {
    /// Where it is held: the grip location when the accessory names one, and its anchor otherwise.
    pub(crate) grip: wxr::Pose,
    /// Where it points: the aim location when the accessory names one, and the grip's own place otherwise.
    pub(crate) aim: wxr::Pose,
    /// Its speed, when ARKit had an anchor for this controller; a pad nothing tracks has no velocity rather
    /// than a zero one, because zero is a claim that it is holding still.
    pub(crate) motion: Option<wxr::Motion>,
    pub(crate) tracked: bool,
    /// Whether that place is a guess because only the orientation is tracked.
    pub(crate) emulated_position: bool,
    /// Whether a hand is holding it, from `ar_accessory_anchor_is_held` - `None` for a controller with no
    /// anchor, because nothing said it was set down.
    pub(crate) held: Option<bool>,
    pub(crate) handedness: wxr::Handedness,
}

impl Place {
    /// The place of a controller nothing tracks: no pose, no speed, and not a hand.
    const UNPLACED: Self = Self {
        grip: wxr::Pose::IDENTITY,
        aim: wxr::Pose::IDENTITY,
        motion: None,
        tracked: false,
        emulated_position: false,
        held: None,
        handedness: wxr::Handedness::Unknown,
    };
}

impl AccessoryPose {
    /// This place in the caller's space, with the two facts a source carries beside it.
    fn place(&self, space_from_world: Mat4) -> Place {
        Place {
            grip: wxr_render::pose_from_transform(space_from_world * self.grip),
            aim: wxr_render::pose_from_transform(space_from_world * self.aim),
            motion: Some(wxr::Motion {
                // A velocity is a direction per second, so it is turned with the pose and not moved with
                // it. Every space this backend reports in is rigid, so there is no scale to undo.
                linear_velocity: Some(space_from_world.transform_vector3(self.linear_velocity)),
                // The accessory's own axes travel with it, so a speed about them reads the same however
                // the caller's space is turned.
                angular_velocity: Some(self.angular_velocity),
            }),
            tracked: self.tracked,
            emulated_position: self.emulated_position,
            held: Some(self.held),
            handedness: self.handedness,
        }
    }
}

/// A device's place, from the poses ARKit reported: the anchor whose device this is, or the unplaced source a
/// pad is when nothing tracks it.
///
/// The lookup is a pointer comparison because the `GCDevice` is the only name the two frameworks share - see
/// [`AccessoryPose::device`] - and a device with no accessory of its own is not an error: it is a pad ARKit
/// has nothing to say about.
pub(crate) fn controller_place(
    places: &[AccessoryPose],
    device: &ProtocolObject<dyn GCDevice>,
    space_from_world: Mat4,
) -> Place {
    let address = std::ptr::from_ref(device).cast::<()>() as usize;
    places
        .iter()
        .find(|place| place.device == address)
        .map_or(Place::UNPLACED, |place| place.place(space_from_world))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three answers ARKit can give, and the core's word for each: a held pad is the hand that holds it,
    /// and one nobody holds and nobody made for a hand is not a hand at all.
    #[test]
    fn a_held_accessory_is_the_hand_holding_it() {
        use ar_accessory_chirality_t as chirality;
        assert_eq!(
            handedness_of(chirality::unspecified, chirality::unspecified),
            wxr::Handedness::Unknown
        );
        assert_eq!(
            handedness_of(chirality::left, chirality::unspecified),
            wxr::Handedness::Left
        );
        assert_eq!(
            handedness_of(chirality::right, chirality::unspecified),
            wxr::Handedness::Right
        );
    }

    /// A pad set down is still the pad it is: the held answer is empty and the answer about the hardware is
    /// not, so the source an app binds by keeps its hand while it rests on a table.
    #[test]
    fn a_pad_that_is_set_down_keeps_the_hand_it_is_made_for() {
        use ar_accessory_chirality_t as chirality;
        assert_eq!(
            handedness_of(chirality::unspecified, chirality::right),
            wxr::Handedness::Right
        );
        assert_eq!(
            handedness_of(chirality::unspecified, chirality::left),
            wxr::Handedness::Left
        );
    }

    /// Held beats designed: a non-handed pad picked up in the left hand is the left source, and so is a
    /// right-handed accessory held on the wrong side.
    #[test]
    fn the_hand_holding_it_wins_over_the_hand_it_was_made_for() {
        use ar_accessory_chirality_t as chirality;
        assert_eq!(
            handedness_of(chirality::left, chirality::right),
            wxr::Handedness::Left
        );
    }

    /// A velocity belongs to the direction the place is read in: turning the caller's space turns a speed
    /// with it, and leaves a speed about the accessory's own axes alone, because those axes turn with it.
    #[test]
    fn a_turned_space_turns_a_velocity_and_not_an_axis() {
        let pose = AccessoryPose {
            device: 0,
            handedness: wxr::Handedness::Right,
            tracked: true,
            emulated_position: false,
            held: true,
            grip: Mat4::IDENTITY,
            aim: Mat4::from_translation(wxr::glam::Vec3::Z),
            linear_velocity: wxr::glam::Vec3::X,
            angular_velocity: wxr::glam::Vec3::Y,
        };
        let place = pose.place(Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let motion = place.motion.expect("an anchor is a place with a speed");
        assert!(
            motion
                .linear_velocity
                .expect("the linear half")
                .abs_diff_eq(wxr::glam::Vec3::NEG_Z, 1e-6)
        );
        assert_eq!(motion.angular_velocity, Some(wxr::glam::Vec3::Y));
        assert!(place.tracked);
        assert!(!place.emulated_position);
        assert_eq!(place.held, Some(true), "the anchor said a hand was on it");
        assert_eq!(place.handedness, wxr::Handedness::Right);
    }

    /// Two places, not one: the grip and the aim keep their own transforms, and the caller's space moves
    /// both rather than flattening them to a single pose.
    #[test]
    fn a_grip_and_an_aim_keep_their_own_places() {
        let pose = AccessoryPose {
            device: 0,
            handedness: wxr::Handedness::Unknown,
            tracked: true,
            emulated_position: false,
            held: false,
            grip: Mat4::IDENTITY,
            aim: Mat4::from_translation(wxr::glam::Vec3::Z),
            linear_velocity: wxr::glam::Vec3::ZERO,
            angular_velocity: wxr::glam::Vec3::ZERO,
        };
        let place = pose.place(Mat4::from_translation(wxr::glam::Vec3::X));
        assert!(place.grip.position.abs_diff_eq(wxr::glam::Vec3::X, 1e-6));
        assert!(
            place
                .aim
                .position
                .abs_diff_eq(wxr::glam::Vec3::new(1.0, 0.0, 1.0), 1e-6)
        );
    }

    /// What the four tracking states mean to the core's two flags: an untracked anchor is not tracked, an
    /// orientation-only one is tracked with a guessed place, and a low-accuracy position is still measured.
    #[test]
    fn only_an_orientation_only_anchor_has_a_guessed_place() {
        use ar_accessory_anchor_tracking_state_t as tracking;
        let states = [
            (tracking::untracked, false),
            (tracking::orientation_tracked, true),
            (tracking::position_orientation_tracked, false),
            (tracking::position_orientation_tracked_low_accuracy, false),
        ];
        for (state, guessed) in states {
            assert_eq!(emulated_position_of(state), guessed);
        }
    }
}
