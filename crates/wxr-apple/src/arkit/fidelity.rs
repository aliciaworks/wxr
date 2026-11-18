//! ARKit's visual fidelity provider, of which this session asks one question: where the wearer's eyes are
//! relative to the display's own range.
//!
//! The provider monitors more than that - whether a promised field of view is actually visible, with an
//! anchor for the boundary - and neither half is asked for here. A field of view is a promise an app made
//! and this core has no word for one, while device fit is a fact about the moment that a session can use
//! without having promised anything. The configuration's other option, system coaching alerts, stays off:
//! the display drawing its own notification over a session that only wanted a hint is not this crate's
//! decision to make.
//!
//! **The provider arrived in visionOS 27**, and ARKit's older systems have neither the symbol nor the
//! question. [`objc2::available!`] answers that before anything is called, so a session on a system without
//! it is a session whose [`wxr::Features::DEVICE_FIT`] is simply absent - the same shape as a provider this
//! device does not support.
//!
//! **Push becomes pull** the way it does for the room's light: ARKit calls the handler on a queue of this
//! backend's own when it has measured again, the handler keeps the answer, and a frame reads it with a lock
//! and a copy. The slot is leaked for the reason every handler context here is - ARKit holds the pointer for
//! as long as it likes, and there is no point at which it can be shown to be finished with it.

use std::ffi::c_void;
use std::sync::Mutex;

use dispatch2::{DispatchQueue, DispatchRetained};
use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_data_providers_t, ar_device_fit_status_t, ar_visual_fidelity_configuration_t,
    ar_visual_fidelity_data_t, ar_visual_fidelity_provider_t,
};

use super::as_data_provider;

/// Device fit: the provider, and the one answer it has to give.
pub(super) struct Fidelity {
    /// Kept for its lifetime: releasing the provider is how the session stops being told anything.
    _provider: Retained<ar_visual_fidelity_provider_t>,
    /// Kept because ARKit only holds a queue for as long as it is calling on it.
    _queue: DispatchRetained<DispatchQueue>,
    /// What the last measurement said, and `Unknown` until the first one arrives.
    fit: &'static Mutex<wxr::DeviceFit>,
}

impl Fidelity {
    /// The fit ARKit measured most recently.
    ///
    /// A copy, like every other read here. A poisoned slot means a writer panicked, and the only writer is a
    /// handler that cannot - so this is unreachable, and a frame is the wrong place to find that out.
    pub(super) fn fit(&self) -> wxr::DeviceFit {
        *self
            .fit
            .lock()
            .expect("the fit slot is only ever written by a handler that cannot panic")
    }

    /// Start the provider and the queue its handler runs on, or `None` on a system that has neither.
    ///
    /// # Safety
    ///
    /// `providers` must be a live collection.
    pub(super) unsafe fn start(providers: &ar_data_providers_t) -> Option<Self> {
        // The order matters: the two calls below are visionOS 27 symbols, and asking the OS first means
        // neither is ever called on a system that does not have them.
        if !objc2::available!(visionos = 27.0) {
            return None;
        }
        // SAFETY: as the other providers - the calls build ARKit objects, and the provider is added to the
        // collection the session retains.
        unsafe {
            if !ar_visual_fidelity_provider_t::is_supported() {
                return None;
            }
            let configuration = ar_visual_fidelity_configuration_t::new();
            // The only measurement this session has a question for.
            ar_visual_fidelity_configuration_t::set_request_device_fit_updates(
                &configuration,
                true,
            );
            let provider = ar_visual_fidelity_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            // Serial, and not the main queue: a measurement landing behind a frame's worth of work is a
            // measurement the frame that wanted it never sees.
            let queue = DispatchQueue::new(Some(c"wxr.fidelity"), None);
            let fit: &'static Mutex<wxr::DeviceFit> =
                Box::leak(Box::new(Mutex::new(wxr::DeviceFit::Unknown)));
            ar_visual_fidelity_provider_t::set_update_handler_f(
                &provider,
                Some(&queue),
                fit as *const _ as *mut c_void,
                Some(fit_updated),
            );
            Some(Self {
                _provider: provider,
                _queue: queue,
                fit,
            })
        }
    }
}

/// The core's word for one of ARKit's statuses.
fn fit_of(status: ar_device_fit_status_t) -> wxr::DeviceFit {
    match status {
        ar_device_fit_status_t::valid => wxr::DeviceFit::Valid,
        ar_device_fit_status_t::eyes_above => wxr::DeviceFit::EyesAbove,
        ar_device_fit_status_t::eyes_below => wxr::DeviceFit::EyesBelow,
        ar_device_fit_status_t::eyes_left => wxr::DeviceFit::EyesLeft,
        ar_device_fit_status_t::eyes_right => wxr::DeviceFit::EyesRight,
        ar_device_fit_status_t::eyes_forward => wxr::DeviceFit::EyesForward,
        ar_device_fit_status_t::eyes_backward => wxr::DeviceFit::EyesBackward,
        // `unknown`, and anything ARKit adds later that this build has no word for. Both are "not a place
        // the app can act on", which is what the core's `Unknown` says.
        _ => wxr::DeviceFit::Unknown,
    }
}

/// ARKit's handler: a new measurement, which is one status in a slot.
///
/// # Safety
///
/// Called by ARKit with the context [`Fidelity::start`] passed, which is the address of a slot that is never
/// freed, and with a live data object.
unsafe extern "C-unwind" fn fit_updated(context: *mut c_void, data: &ar_visual_fidelity_data_t) {
    // SAFETY: the context is the address `start` leaked, which lives as long as the process does.
    let slot = unsafe { &*context.cast::<Mutex<wxr::DeviceFit>>() };
    // SAFETY: the caller guarantees the data object.
    let fit = fit_of(unsafe { ar_visual_fidelity_data_t::device_fit_status(data) });
    *slot
        .lock()
        .expect("the fit slot is only ever written by a handler that cannot panic") = fit;
}
