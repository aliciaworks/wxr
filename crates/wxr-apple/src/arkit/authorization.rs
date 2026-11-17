//! What ARKit says this app is allowed to see, and the two directions it arrives in.
//!
//! A refused provider and an absent one look the same from the session above: both are an `Option` that is
//! `None`, both leave [`wxr::Features`] without the bit, and an app that only counted providers would tell a
//! wearer their headset cannot trace the room when the truth is that they said no once. ARKit knows the
//! difference - `ar_authorization_status_t` is `not_determined`, `allowed` or `denied` - so this asks it, and
//! [`wxr::Authorization`] is where the answer goes.
//!
//! **Two directions, one slot.** The query at startup answers on an arbitrary queue, and the update handler
//! fires when the wearer changes their mind in the settings app; both write into one leaked mutex, for the
//! reason the light estimate beside this does - ARKit holds a context for as long as it likes, and a frame
//! reads the slot as a lock and a copy rather than waiting on a provider.
//!
//! **Which permission a capability needs is ARKit's answer, not this crate's.** Every provider names one -
//! `required_authorization_type` - so [`required`] asks them instead of repeating what today's headers say,
//! and every type ARKit names is queried: a provider that starts needing camera access is then a session
//! that already has the answer.

use std::ffi::c_void;
use std::sync::Mutex;

use dispatch2::{DispatchQueue, DispatchRetained};
use objc2_ar_kit::{
    ar_authorization_result_t, ar_authorization_results_t, ar_authorization_status_t,
    ar_authorization_type_t, ar_environment_light_estimation_provider_t, ar_error_t,
    ar_hand_tracking_provider_t, ar_plane_detection_provider_t, ar_scene_reconstruction_provider_t,
    ar_session_t, ar_world_tracking_provider_t,
};

/// The permissions this session asks about: every type ARKit names today.
///
/// Not only the ones this backend turns out to use, because which provider needs which is the provider's
/// answer and this crate does not get a vote. A query is not a prompt, so asking about one this device has
/// no provider for costs a message and none of the wearer's attention.
const ASKED: [ar_authorization_type_t; 4] = [
    ar_authorization_type_t::hand_tracking,
    ar_authorization_type_t::world_sensing,
    ar_authorization_type_t::camera_access,
    ar_authorization_type_t::accessory_tracking,
];

/// One answer per [`ASKED`], `NotDetermined` until ARKit says otherwise.
type Held = [wxr::Authorization; ASKED.len()];

/// The slot ARKit writes into, and the queue it writes on.
pub(super) struct Permissions {
    /// Kept because ARKit only holds a queue for as long as it is calling on it.
    _queue: DispatchRetained<DispatchQueue>,
    held: &'static Mutex<Held>,
}

impl Permissions {
    /// Ask ARKit what it has already decided, and keep listening for a change of mind.
    ///
    /// `session` must be a live one. Both calls outlive this, so the slot is leaked for the reason the light
    /// estimate's is: there is no point at which ARKit says it is finished with a context pointer, and the
    /// other end of that trade is a handler writing into a session that has already been dropped.
    pub(super) fn start(session: &ar_session_t) -> Self {
        // A queue of this backend's own rather than the main one, which is where an app waits for the
        // compositor - an answer that has to queue behind a frame is one that arrives after it was wanted.
        let queue = DispatchQueue::new(Some(c"wxr.authorization"), None);
        let held: &'static Mutex<Held> = Box::leak(Box::new(Mutex::new(
            [wxr::Authorization::NotDetermined; ASKED.len()],
        )));
        let context = held as *const _ as *mut c_void;
        // SAFETY: the session is live, the context is the leaked slot above, and both callbacks do nothing
        // but take that lock and write one entry.
        unsafe {
            ar_session_t::set_authorization_update_handler_f(
                session,
                Some(&queue),
                context,
                Some(updated),
            );
            ar_session_t::query_authorization_results_f(session, asked(), context, queried);
        }
        Self {
            _queue: queue,
            held,
        }
    }

    /// What ARKit has said about `feature`, and about every permission it needs.
    ///
    /// A set answers for all of it, and the cautious answer wins: one refusal in a set is a refusal, and one
    /// permission nobody has been asked about yet is not a yes.
    pub(super) fn of(&self, feature: wxr::Features) -> wxr::Authorization {
        let held = self
            .held
            .lock()
            .expect("the authorization slot is only written by handlers that cannot panic");
        let mut answer = wxr::Authorization::Granted;
        for (bit, permission) in required() {
            if feature.contains(bit)
                && let Some(index) = slot(permission)
            {
                answer = narrower(answer, held[index]);
            }
        }
        answer
    }
}

/// Which of the core's capabilities need a permission, and ARKit's own answer as to which.
fn required() -> [(wxr::Features, ar_authorization_type_t); 6] {
    // SAFETY: each is a constant the framework answers with no preconditions.
    unsafe {
        [
            (
                wxr::Features::HAND_TRACKING,
                ar_hand_tracking_provider_t::required_authorization_type(),
            ),
            (
                wxr::Features::PLANES,
                ar_plane_detection_provider_t::required_authorization_type(),
            ),
            (
                wxr::Features::MESH,
                ar_scene_reconstruction_provider_t::required_authorization_type(),
            ),
            (
                wxr::Features::ANCHORS,
                ar_world_tracking_provider_t::required_authorization_type(),
            ),
            (
                wxr::Features::LIGHT_ESTIMATION,
                ar_environment_light_estimation_provider_t::required_authorization_type(),
            ),
            // A cast is not a provider: it reads the two room providers, and both answer world sensing
            // today - so the permission is the room's rather than a fourth question.
            (
                wxr::Features::HIT_TEST,
                ar_plane_detection_provider_t::required_authorization_type(),
            ),
        ]
    }
}

/// The element of [`ASKED`] that `permission` is, and `None` for a type nothing here asked about.
fn slot(permission: ar_authorization_type_t) -> Option<usize> {
    ASKED.iter().position(|asked| *asked == permission)
}

/// Every type in [`ASKED`] at once, which is what the query takes.
fn asked() -> ar_authorization_type_t {
    let mut mask = ar_authorization_type_t::none.0;
    for permission in ASKED {
        mask |= permission.0;
    }
    ar_authorization_type_t(mask)
}

/// The more cautious of two answers, which is what a question about several permissions comes to.
fn narrower(a: wxr::Authorization, b: wxr::Authorization) -> wxr::Authorization {
    use wxr::Authorization::{Denied, Granted, NotDetermined};
    match (a, b) {
        (Denied, _) | (_, Denied) => Denied,
        (NotDetermined, _) | (_, NotDetermined) => NotDetermined,
        (Granted, Granted) => Granted,
    }
}

/// ARKit's status as the core's three.
fn status(status: ar_authorization_status_t) -> wxr::Authorization {
    if status == ar_authorization_status_t::allowed {
        wxr::Authorization::Granted
    } else if status == ar_authorization_status_t::denied {
        wxr::Authorization::Denied
    } else {
        // `not_determined`, and anything a later SDK adds: a status this does not know is not a yes.
        wxr::Authorization::NotDetermined
    }
}

/// Record one of ARKit's answers, wherever it arrived from.
fn record(held: &mut Held, result: &ar_authorization_result_t) {
    // SAFETY: the result is live for the call, and both getters only read it.
    let (permission, answer) = unsafe {
        (
            ar_authorization_result_t::authorization_type(result),
            ar_authorization_result_t::status(result),
        )
    };
    if let Some(index) = slot(permission) {
        held[index] = status(answer);
    }
}

/// The query's answer, one entry at a time.
unsafe extern "C-unwind" fn queried(
    context: *mut c_void,
    results: &ar_authorization_results_t,
    error: Option<&ar_error_t>,
) {
    if error.is_some() {
        // Nothing was asked, so nothing is known: the slot keeps `NotDetermined`, which is what it says
        // until somebody who can answer does.
        return;
    }
    // SAFETY: the context is the slot `start` leaked, which lives as long as the process does.
    let held = unsafe { &*context.cast::<Mutex<Held>>() };
    let mut held = held
        .lock()
        .expect("the authorization slot is only written by handlers that cannot panic");
    // SAFETY: the collection is live for the call, and the enumerator only reads the result ARKit gives it.
    unsafe {
        ar_authorization_results_t::enumerate_results_f(
            results,
            std::ptr::from_mut(&mut *held).cast(),
            collected,
        )
    };
}

/// One result of that answer, and `true` because every one of them is wanted.
unsafe extern "C-unwind" fn collected(
    context: *mut c_void,
    result: &ar_authorization_result_t,
) -> bool {
    // SAFETY: the context is the guard `queried` holds for the call.
    let held = unsafe { &mut *context.cast::<Held>() };
    record(held, result);
    true
}

/// A change of mind in the settings app.
unsafe extern "C-unwind" fn updated(context: *mut c_void, result: &ar_authorization_result_t) {
    // SAFETY: the context is the slot `start` leaked, which lives as long as the process does.
    let held = unsafe { &*context.cast::<Mutex<Held>>() };
    let mut held = held
        .lock()
        .expect("the authorization slot is only written by handlers that cannot panic");
    record(&mut held, result);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wxr::Authorization::{Denied, Granted, NotDetermined};

    #[test]
    fn a_set_is_as_granted_as_its_least_granted_member() {
        assert_eq!(narrower(Granted, Granted), Granted);
        assert_eq!(narrower(Granted, Denied), Denied);
        assert_eq!(narrower(Denied, Granted), Denied);
        assert_eq!(narrower(Denied, Denied), Denied);
        // Not asked is not a yes, and it is not a refusal either.
        assert_eq!(narrower(Granted, NotDetermined), NotDetermined);
        assert_eq!(narrower(NotDetermined, Denied), Denied);
    }

    #[test]
    fn the_three_arkit_statuses_are_the_three_answers() {
        assert_eq!(status(ar_authorization_status_t::allowed), Granted);
        assert_eq!(status(ar_authorization_status_t::denied), Denied);
        assert_eq!(
            status(ar_authorization_status_t::not_determined),
            NotDetermined
        );
    }

    #[test]
    fn every_type_asked_about_has_a_slot_and_the_mask_is_all_of_them() {
        for (index, permission) in ASKED.iter().enumerate() {
            assert_eq!(slot(*permission), Some(index));
        }
        assert_eq!(slot(ar_authorization_type_t::none), None);
        assert_eq!(asked().0, ASKED.iter().fold(0, |mask, one| mask | one.0));
        // The mask is not `none`, which is the one value a query could be made with and answer nothing.
        assert_ne!(asked().0, ar_authorization_type_t::none.0);
    }
}
