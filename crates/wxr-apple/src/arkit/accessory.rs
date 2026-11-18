//! Accessories: where a controller is, which is the half GameController does not have.
//!
//! A pad reports buttons and an actuator; ARKit's accessory tracking is what says where it is in the room.
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
//! **One transform, two poses.** The anchor's transform, asked for *uncorrected* - the plain call is the one
//! the compositor renders with, moved to land over the physical accessory in passthrough - is its place and
//! orientation, and the core's grip and aim are both it, the way they are for a hand. The named
//! locations ARKit also offers - `ar_accessory_location_name_aim` and the grip surface - are where a stylus
//! points and where a pad is held rather than where the accessory is, and a core with one pair of poses has
//! nowhere to put the difference. Reading them is `..._get_anchor_from_location_transform_with_correction`,
//! a `simd` call this backend would need another shim for; the honest simplification is that the aim comes
//! out of the same anchor as the grip, and the docs say so.
//!
//! **And how fast it is going.** The anchor also carries ARKit's estimate of its velocity, in metres per
//! second along ARKit's axes, and of its angular velocity, in radians per second about its own; both go into
//! [`wxr::InputSource::motion`]. A velocity is a direction per second, so it turns with the caller's space
//! and is not moved by it, and a speed about the accessory's own axes reads the same however that space is
//! turned.

use std::ffi::c_void;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_ar_kit::{
    ar_accessories_t, ar_accessory_anchor_t, ar_accessory_anchor_tracking_state_t,
    ar_accessory_anchors_t, ar_accessory_chirality_t, ar_accessory_t,
    ar_accessory_tracking_configuration_t, ar_accessory_tracking_provider_t, ar_data_providers_t,
    ar_error_t, ar_session_t, ar_transform_correction_t,
};
use objc2_game_controller::{GCController, GCDevice};
use wxr::glam::Mat4;

use super::{ArKit, as_data_provider};
use crate::sys;

/// Where one accessory is, how it is moving, and which controller it is.
pub(crate) struct AccessoryPose {
    /// The `GCDevice` this anchor was loaded from, as the address GameController knows it by. The pointer is
    /// the only name the two frameworks share, and comparing it is how a controller finds its own place.
    pub(crate) device: usize,
    /// Which hand the accessory is being held in, or `Unknown` for one that is not held.
    pub(crate) handedness: wxr::Handedness,
    /// Whether ARKit is tracking it right now: an accessory on a table is still placed, one the runtime has
    /// lost is not.
    pub(crate) tracked: bool,
    /// Whether the *position* is a guess because only the orientation is tracked, which is the core's
    /// `emulated_position` and a different question from `tracked`.
    pub(crate) emulated_position: bool,
    /// Its place and orientation, in ARKit's world, unaltered by the rendering correction.
    pub(crate) transform: Mat4,
    /// Metres per second along ARKit's axes, estimated by the runtime rather than differenced here.
    pub(crate) linear_velocity: wxr::glam::Vec3,
    /// Radians per second about the accessory's own axes.
    pub(crate) angular_velocity: wxr::glam::Vec3,
}

/// Accessory tracking: the provider, the set it tracks, and the accessories that have loaded but are not in
/// that set yet.
pub(super) struct Accessories {
    /// `None` until an accessory has loaded: a configuration names the set it tracks at creation.
    provider: Option<Retained<ar_accessory_tracking_provider_t>>,
    /// The set the provider is tracking, kept and added to rather than rebuilt - a later arrival is added to
    /// this and handed back to the provider on a system that can update it.
    tracked: Retained<ar_accessories_t>,
    /// Addresses of loaded accessories, each a `+1`, waiting for [`Accessories::provider`] to own them.
    loaded: &'static Mutex<Vec<usize>>,
}

impl Accessories {
    /// The provider and the set to fill, or `None` on a system without accessory tracking.
    ///
    /// `ar_accessory_tracking_provider_is_supported` is a visionOS 26.0 symbol, so the availability question
    /// comes first: on anything older neither it nor anything below is ever called.
    pub(super) fn start() -> Option<Self> {
        if !objc2::available!(visionos = 26.0) {
            return None;
        }
        // SAFETY: the symbol exists on every system this build can reach past the guard, and it asks the
        // framework a question about the device with no preconditions.
        let supported = unsafe { ar_accessory_tracking_provider_t::is_supported() };
        if !supported {
            return None;
        }
        // SAFETY: the creator returns `+1`, which `Retained` owns from here. The slot is leaked because the
        // load completion holds its address for as long as ARKit likes.
        let tracked = unsafe { ar_accessories_t::new() };
        Some(Self {
            provider: None,
            tracked,
            loaded: Box::leak(Box::new(Mutex::new(Vec::new()))),
        })
    }

    /// Ask ARKit to load the accessory behind one controller.
    ///
    /// Once per `GCDevice`, because a load is a request rather than a state: the caller asks when it sees a
    /// controller it has not seen before, and the answer arrives on the callback below.
    pub(super) fn load(&self, device: &ProtocolObject<dyn GCDevice>) {
        // SAFETY: the device is live, the context is the leaked slot which outlives any completion, and the
        // completion touches nothing but that slot.
        unsafe {
            ar_accessory_t::load_from_device_f(
                device,
                self.loaded as *const Mutex<Vec<usize>> as *mut c_void,
                loaded,
            );
        }
    }

    /// Start the provider once accessories have loaded, and add later ones to a provider that is already up.
    ///
    /// `session` and `providers` are the live pair the rest of the session runs with: ARKit takes a provider
    /// into a running session by being run again with the collection that now names it.
    pub(super) fn provider(
        &mut self,
        session: &ar_session_t,
        providers: &ar_data_providers_t,
    ) -> Option<&ar_accessory_tracking_provider_t> {
        let fresh = self.take_loaded();
        // SAFETY: both collections are live, and each accessory is a `+1` this call now owns.
        for accessory in &fresh {
            unsafe { ar_accessories_t::add_accessory(&self.tracked, accessory) };
        }
        match &self.provider {
            // Nothing has loaded yet, so there is no set to configure a provider with.
            None if fresh.is_empty() => return None,
            None => {
                // SAFETY: the calls build ARKit objects from a live set, and the provider is handed to the
                // collection the session retains before the session is run with it.
                let provider = unsafe {
                    let configuration = ar_accessory_tracking_configuration_t::new();
                    ar_accessory_tracking_configuration_t::set_accessories(
                        &configuration,
                        &self.tracked,
                    );
                    let provider = ar_accessory_tracking_provider_t::new(&configuration);
                    ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
                    ar_session_t::run(session, providers);
                    provider
                };
                self.provider = Some(provider);
            }
            // On 26 the set a provider was made with is the set it tracks, so a later arrival waits.
            Some(_) if fresh.is_empty() => {}
            Some(provider) if objc2::available!(visionos = 27.0) => {
                // SAFETY: the provider is live, the set is live, and the completion does nothing with its
                // context - which is null - and only decides when the request is over.
                unsafe {
                    ar_accessory_tracking_provider_t::update_accessories_f(
                        provider,
                        &self.tracked,
                        std::ptr::null_mut(),
                        updated,
                    );
                }
            }
            Some(_) => {}
        }
        self.provider.as_deref()
    }

    /// Where the accessories are, as the latest anchors the provider holds.
    ///
    /// A pull rather than an update stream: the anchors are current when asked, and a frame wants this
    /// frame's place rather than the last push. An accessory the provider tracks but ARKit is not tracking
    /// still has an anchor, with `tracked` false - which is what the core's own flag says.
    pub(super) fn poses(&self) -> Vec<AccessoryPose> {
        let Some(provider) = &self.provider else {
            return Vec::new();
        };
        // SAFETY: the provider is live, and the enumeration only reads the anchors it is handed.
        unsafe {
            let anchors = ar_accessory_tracking_provider_t::latest_anchors(provider);
            let mut out = Vec::new();
            ar_accessory_anchors_t::enumerate_anchors_f(
                &anchors,
                std::ptr::from_mut(&mut out).cast::<c_void>(),
                collect,
            );
            out
        }
    }

    /// Turn every address a load callback stored back into the object it is the address of.
    ///
    /// The mailbox is emptied, so each `+1` comes back exactly once however many frames it took to arrive.
    fn take_loaded(&self) -> Vec<Retained<ar_accessory_t>> {
        let addresses = std::mem::take(
            &mut *self
                .loaded
                .lock()
                .expect("the accessory mailbox is only written by a callback that cannot panic"),
        );
        addresses
            .into_iter()
            // SAFETY: each address is one `Retained::into_raw` from the callback, turned back here on the
            // thread that owns the set and never twice.
            .filter_map(|address| unsafe { Retained::from_raw(address as *mut ar_accessory_t) })
            .collect()
    }
}

/// ARKit's completion: retain the accessory and record where it is, since a `Retained` cannot cross a thread
/// here and an address can.
///
/// # Safety
///
/// Called by ARKit with the context [`Accessories::load`] passed, which is a leaked slot, and with an
/// accessory that is live for the call or absent on a failed load.
unsafe extern "C-unwind" fn loaded(
    context: *mut c_void,
    _device: &ProtocolObject<dyn GCDevice>,
    successful: bool,
    _error: Option<&ar_error_t>,
    accessory: Option<&ar_accessory_t>,
) {
    let Some(accessory) = accessory.filter(|_| successful) else {
        return;
    };
    // SAFETY: the context is the address `load` leaked, which lives as long as the process does.
    let slot = unsafe { &*context.cast::<Mutex<Vec<usize>>>() };
    // SAFETY: the accessory is live for the call, and this takes the `+1` that crosses to the session's
    // thread. A null answer is a failed load, which `successful` already covers.
    let Some(accessory) =
        (unsafe { Retained::retain(accessory as *const ar_accessory_t as *mut ar_accessory_t) })
    else {
        return;
    };
    slot.lock()
        .expect("the accessory mailbox is only written by a callback that cannot panic")
        .push(Retained::into_raw(accessory) as usize);
}

/// ARKit's answer to an accessory update: the set it now tracks is the one it was handed, and the reason a
/// request failed is not a place any frame can be drawn from.
///
/// # Safety
///
/// Called by ARKit with the context [`Accessories::provider`] passed, which is null, and with an error that
/// is live for the call when there is one.
unsafe extern "C-unwind" fn updated(
    _context: *mut c_void,
    _success: bool,
    _error: Option<&ar_error_t>,
) {
}

/// The enumerator [`Accessories::poses`] hands ARKit: one anchor in, one [`AccessoryPose`] out.
///
/// # Safety
///
/// Called by ARKit with the context [`Accessories::poses`] passed, which is a vector that outlives the
/// enumeration, and with an anchor that is live for the call.
unsafe extern "C-unwind" fn collect(context: *mut c_void, anchor: &ar_accessory_anchor_t) -> bool {
    // SAFETY: the context is the vector `poses` passed, and the enumeration is synchronous with it.
    let out = unsafe { &mut *context.cast::<Vec<AccessoryPose>>() };
    // SAFETY: the anchor is live for the call, and every call below reads it rather than taking it.
    unsafe {
        let accessory = ar_accessory_anchor_t::accessory(anchor);
        // A `GCDevice` is a controller's other half, and an accessory with no source device has no
        // controller to belong to; a pose nothing can be matched to is one no caller would see.
        let Some(device) = ar_accessory_t::source_device(&accessory) else {
            return true;
        };
        let anchor_address = std::ptr::from_ref(anchor).cast::<c_void>();
        out.push(AccessoryPose {
            device: Retained::as_ptr(&device).cast::<()>() as usize,
            handedness: handedness_of(ar_accessory_anchor_t::held_chirality(anchor)),
            tracked: ar_accessory_anchor_t::is_tracked(anchor),
            emulated_position: emulated_position_of(ar_accessory_anchor_t::tracking_state(anchor)),
            // Not the generic anchor call: ARKit's plain accessory transform is the *rendered* correction,
            // and a pose an app points with is where the accessory actually is.
            transform: Mat4::from_cols_array(
                &sys::ar_accessory_anchor_get_origin_from_anchor_transform_with_correction(
                    anchor_address,
                    ar_transform_correction_t::none.0,
                )
                .0,
            ),
            // The runtime's own estimate, which is why it is read rather than differenced between frames:
            // an accessory ARKit is tracking has a speed ARKit knows.
            linear_velocity: wxr::glam::Vec3::from_array(sys::ar_accessory_anchor_get_velocity(
                anchor_address,
            )),
            angular_velocity: wxr::glam::Vec3::from_array(
                sys::ar_accessory_anchor_get_angular_velocity(anchor_address),
            ),
        });
    }
    // Every accessory in the collection is one this session asked about, so none is skipped.
    true
}

/// Whether the position is a guess, from ARKit's richer answer about what it tracks: orientation alone is
/// the three-degrees-of-freedom case the core's flag is about, and a low accuracy position is still measured.
fn emulated_position_of(state: ar_accessory_anchor_tracking_state_t) -> bool {
    state == ar_accessory_anchor_tracking_state_t::orientation_tracked
}

/// Which hand an accessory is in, from ARKit's answer about the anchor that was held.
fn handedness_of(chirality: ar_accessory_chirality_t) -> wxr::Handedness {
    match chirality {
        ar_accessory_chirality_t::left => wxr::Handedness::Left,
        ar_accessory_chirality_t::right => wxr::Handedness::Right,
        // `unspecified`, and anything ARKit adds later: a non-handed accessory, or one on a table and in
        // nobody's hand.
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
    /// until ARKit has answered a load.
    pub(crate) fn accessory_poses(&mut self) -> Vec<AccessoryPose> {
        let Some(accessories) = &mut self.accessories else {
            return Vec::new();
        };
        accessories.provider(&self._session, &self._providers);
        accessories.poses()
    }
}

/// Where a controller is and how it moves, in the space being reported in.
pub(crate) struct Place {
    /// The pose the core's grip and aim both take - one anchor, and no second point to aim from.
    pub(crate) pose: wxr::Pose,
    /// Its speed, when ARKit had an anchor for this controller; a pad nothing tracks has no velocity rather
    /// than a zero one, because zero is a claim that it is holding still.
    pub(crate) motion: Option<wxr::Motion>,
    pub(crate) tracked: bool,
    /// Whether that place is a guess because only the orientation is tracked.
    pub(crate) emulated_position: bool,
    pub(crate) handedness: wxr::Handedness,
}

impl Place {
    /// The place of a controller nothing tracks: no pose, no speed, and not a hand.
    const UNPLACED: Self = Self {
        pose: wxr::Pose::IDENTITY,
        motion: None,
        tracked: false,
        emulated_position: false,
        handedness: wxr::Handedness::Unknown,
    };
}

impl AccessoryPose {
    /// This place in the caller's space, with the two facts a source carries beside it.
    fn place(&self, space_from_world: Mat4) -> Place {
        Place {
            pose: wxr_render::pose_from_transform(space_from_world * self.transform),
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
            handedness: self.handedness,
        }
    }
}

/// A controller's place, from the poses ARKit reported: the anchor whose device is this controller, or the
/// unplaced source a pad is when nothing tracks it.
///
/// The lookup is a pointer comparison because the `GCDevice` is the only name the two frameworks share - see
/// [`AccessoryPose::device`] - and a controller with no accessory of its own is not an error: it is a pad
/// ARKit has nothing to say about.
pub(crate) fn controller_place(
    places: &[AccessoryPose],
    controller: &GCController,
    space_from_world: Mat4,
) -> Place {
    let device: &ProtocolObject<dyn GCDevice> = ProtocolObject::from_ref(controller);
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
    /// and one nobody holds is not a hand at all.
    #[test]
    fn a_held_accessory_is_the_hand_holding_it() {
        assert_eq!(
            handedness_of(ar_accessory_chirality_t::unspecified),
            wxr::Handedness::Unknown
        );
        assert_eq!(
            handedness_of(ar_accessory_chirality_t::left),
            wxr::Handedness::Left
        );
        assert_eq!(
            handedness_of(ar_accessory_chirality_t::right),
            wxr::Handedness::Right
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
            transform: Mat4::IDENTITY,
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
        assert_eq!(place.handedness, wxr::Handedness::Right);
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
