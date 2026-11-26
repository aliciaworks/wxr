//! The provider, the set it tracks, and what a frame asks them for.
//!
//! Accessories arrive late and out of order: a load answers on ARKit's queue, the provider has to be built
//! from whatever has arrived, and a frame wants each accessory's places from the anchors the provider holds.
//! That is the whole of this file. What a place *means* to the core - which pose is the grip, which is the
//! aim - is [`super`]'s, and the two meet in [`AccessoryPose`].
//!
//! **The named locations are read once.** A pad labels an aim, a grip and a grip surface; a stylus labels
//! only an aim. The names are a property of the accessory and not of a frame, so `Named` is filled in the
//! load completion, carried across the thread with the accessory, and stored beside the address the anchor
//! reports - a frame then does no string comparison and allocates nothing but the poses.

use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::ptr::NonNull;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_ar_kit::{
    ar_accessories_t, ar_accessory_anchor_t, ar_accessory_anchors_t,
    ar_accessory_location_name_aim, ar_accessory_location_name_grip, ar_accessory_t,
    ar_accessory_tracking_configuration_t, ar_accessory_tracking_provider_t, ar_data_providers_t,
    ar_error_t, ar_session_t, ar_strings_t, ar_transform_correction_t,
};
use objc2_game_controller::GCDevice;
use wxr::glam::Mat4;

use super::super::as_data_provider;
use crate::sys;

/// Accessory tracking: the provider, the set it tracks, and the accessories that have loaded but are not in
/// that set yet.
pub(crate) struct Accessories {
    /// `None` until an accessory has loaded: a configuration names the set it tracks at creation.
    provider: Option<Retained<ar_accessory_tracking_provider_t>>,
    /// The set the provider is tracking, kept and added to rather than rebuilt - a later arrival is added to
    /// this and handed back to the provider on a system that can update it.
    tracked: Retained<ar_accessories_t>,
    /// Addresses of loaded accessories, each a `+1`, waiting for [`Accessories::provider`] to own them.
    loaded: &'static Mutex<Vec<Loaded>>,
    /// Which locations each loaded accessory named, by the address its anchor reports - the accessory both
    /// frameworks know it by, and the key a frame looks a place up with.
    named: HashMap<usize, Named>,
    /// ARKit's own names for the loaded accessories, by the `GCDevice` each came from - the one place a
    /// controller has a name a person reads rather than a product category.
    names: HashMap<usize, String>,
    /// The anchor a prediction is written into, made on the first frame that asks for one: ARKit fills an
    /// anchor of the caller's rather than answering with a fresh one, and a frame predicts one accessory at a
    /// time.
    predicted: Option<Retained<ar_accessory_anchor_t>>,
}

impl Accessories {
    /// The provider and the set to fill, or `None` on a system without accessory tracking.
    ///
    /// `ar_accessory_tracking_provider_is_supported` is a visionOS 26.0 symbol, so the availability question
    /// comes first: on anything older neither it nor anything below is ever called.
    pub(crate) fn start() -> Option<Self> {
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
            named: HashMap::new(),
            names: HashMap::new(),
            predicted: None,
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
                self.loaded as *const Mutex<Vec<Loaded>> as *mut c_void,
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
        for ready in &fresh {
            // SAFETY: both collections are live, and each accessory is a `+1` this call now owns.
            unsafe { ar_accessories_t::add_accessory(&self.tracked, &ready.accessory) };
            self.named.insert(
                Retained::as_ptr(&ready.accessory).cast::<()>() as usize,
                ready.named,
            );
            if let Some(name) = &ready.name {
                self.names.insert(ready.device, name.clone());
            }
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

    /// Where the accessories are, as the latest anchors the provider holds, predicted for an instant.
    ///
    /// A pull rather than an update stream: the anchors are current when asked, and a frame wants this
    /// frame's place rather than the last push. An accessory the provider tracks but ARKit is not tracking
    /// still has an anchor, with `tracked` false - which is what the core's own flag says.
    ///
    /// `at` is the Mach absolute time the frame's trackable anchors are asked about, and every tracked anchor
    /// is predicted to it: the latest anchor is where the accessory was when ARKit last saw it, which is a
    /// frame or more before the picture drawn from it. `None` asks for the latest anchors unchanged.
    pub(super) fn poses(&mut self, at: Option<f64>) -> Vec<AccessoryPose> {
        let Some(provider) = &self.provider else {
            return Vec::new();
        };
        let prediction = at.map(|at| {
            // SAFETY: the anchor creator returns `+1`, which the `Retained` takes from here.
            let out = self
                .predicted
                .get_or_insert_with(|| unsafe { ar_accessory_anchor_t::new() });
            Prediction { provider, at, out }
        });
        // SAFETY: the provider is live, and the enumeration only reads the anchors it is handed - and writes
        // into the one prediction anchor, one accessory at a time.
        unsafe {
            let anchors = ar_accessory_tracking_provider_t::latest_anchors(provider);
            let mut found = Anchors {
                poses: Vec::new(),
                named: &self.named,
                prediction,
            };
            ar_accessory_anchors_t::enumerate_anchors_f(
                &anchors,
                std::ptr::from_mut(&mut found).cast::<c_void>(),
                collect,
            );
            found.poses
        }
    }

    /// Turn every address a load callback stored back into the object it is the address of.
    ///
    /// The mailbox is emptied, so each `+1` comes back exactly once however many frames it took to arrive.
    fn take_loaded(&self) -> Vec<Ready> {
        let loaded = std::mem::take(
            &mut *self
                .loaded
                .lock()
                .expect("the accessory mailbox is only written by a callback that cannot panic"),
        );
        loaded
            .into_iter()
            .filter_map(|loaded| {
                // SAFETY: each address is one `Retained::into_raw` from the callback, turned back here on
                // the thread that owns the set and never twice.
                unsafe { Retained::from_raw(loaded.accessory as *mut ar_accessory_t) }.map(
                    |accessory| Ready {
                        accessory,
                        named: loaded.named,
                        device: loaded.device,
                        name: loaded.name,
                    },
                )
            })
            .collect()
    }

    /// ARKit's name for the accessory behind a controller, when that controller has loaded one and the
    /// accessory named itself.
    pub(crate) fn name(&self, device: usize) -> Option<&str> {
        self.names.get(&device).map(String::as_str)
    }
}

/// A loaded accessory now owned by the session's thread, with what ARKit said about it.
struct Ready {
    accessory: Retained<ar_accessory_t>,
    named: Named,
    /// The `GCDevice` the load was asked for, as the address a source is handed out under.
    device: usize,
    /// ARKit's own name for the accessory, copied while it was live.
    name: Option<String>,
}

/// One loaded accessory on its way to the session's thread: the object itself, as an address, and what ARKit
/// said about the locations it names.
struct Loaded {
    /// The `+1` address of the accessory.
    accessory: usize,
    named: Named,
    /// The `GCDevice` the load was asked for, as the address a source is handed out under.
    device: usize,
    /// ARKit's own name for the accessory, copied while it was live.
    name: Option<String>,
}

/// Which of the core's two named locations an accessory has.
///
/// A pad names a grip and an aim; a stylus names only an aim, which is why the grip falls back to the
/// anchor's own transform rather than to a guessed offset. The grip surface is not one of the two the core
/// has poses for: it is a surface, and neither the grip nor the aim is a plane.
#[derive(Clone, Copy, Default)]
struct Named {
    grip: bool,
    aim: bool,
}

impl Named {
    /// Read the names ARKit reports for an accessory, once, while it is in hand.
    ///
    /// # Safety
    ///
    /// `accessory` must be a live accessory object from the generated crate.
    unsafe fn of(accessory: &ar_accessory_t) -> Self {
        let mut named = Self::default();
        // SAFETY: the accessory is live, the collection the copy returns is `+1` and dropped here, and the
        // callback reads only the strings ARKit hands it.
        unsafe {
            let names = ar_accessory_t::location_names(accessory);
            ar_strings_t::enumerate_strings_f(
                &names,
                std::ptr::from_mut(&mut named).cast::<c_void>(),
                note_location,
            );
        }
        named
    }
}

/// The enumerator `Named::of` hands ARKit: match a name against the two the core has poses for.
///
/// # Safety
///
/// Called by ARKit with the context `Named::of` passed, which outlives the call, and with a NUL-terminated
/// string that is live for it.
unsafe extern "C-unwind" fn note_location(
    context: *mut c_void,
    name: NonNull<std::ffi::c_char>,
) -> bool {
    // SAFETY: the context is the `Named` `of` passed, and every string below is NUL-terminated.
    unsafe {
        let named = &mut *context.cast::<Named>();
        let name = CStr::from_ptr(name.as_ptr());
        // SAFETY: the two statics are Apple's own NUL-terminated names.
        named.grip |= name == CStr::from_ptr(ar_accessory_location_name_grip.as_ptr());
        named.aim |= name == CStr::from_ptr(ar_accessory_location_name_aim.as_ptr());
    }
    // Every name is looked for; stopping early would save nothing on a collection of three strings.
    true
}

/// One frame's prediction: the provider to ask, the time to ask for, and the one anchor answers go into.
#[derive(Clone, Copy)]
struct Prediction<'a> {
    provider: &'a ar_accessory_tracking_provider_t,
    at: f64,
    out: &'a ar_accessory_anchor_t,
}

/// The state one enumeration of anchors reads: where the poses go, what each accessory named, and whether
/// each anchor is predicted before it is read.
struct Anchors<'a> {
    poses: Vec<AccessoryPose>,
    named: &'a HashMap<usize, Named>,
    prediction: Option<Prediction<'a>>,
}

/// One of an accessory's named locations, in ARKit's world.
///
/// `anchor_from_location` is the location's coordinate system inside the anchor's, so the world transform is
/// the anchor's own times it - the multiplication ARKit's name reads from right to left.
///
/// # Safety
///
/// `anchor` must be a live accessory anchor, and `name` one the accessory reported.
unsafe fn location(
    world_from_anchor: Mat4,
    anchor: *const c_void,
    name: NonNull<std::ffi::c_char>,
) -> Mat4 {
    // SAFETY: the caller guarantees both, and the transform is asked for uncorrected like the anchor's own.
    let local = unsafe {
        sys::ar_accessory_anchor_get_anchor_from_location_transform_with_correction(
            anchor,
            name.as_ptr(),
            ar_transform_correction_t::none.0,
        )
    };
    world_from_anchor * Mat4::from_cols_array(&local.0)
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
    device: &ProtocolObject<dyn GCDevice>,
    successful: bool,
    _error: Option<&ar_error_t>,
    accessory: Option<&ar_accessory_t>,
) {
    let Some(accessory) = accessory.filter(|_| successful) else {
        return;
    };
    // SAFETY: the context is the address `load` leaked, which lives as long as the process does.
    let slot = unsafe { &*context.cast::<Mutex<Vec<Loaded>>>() };
    // SAFETY: the accessory is live for the call, and this takes the `+1` that crosses to the session's
    // thread. A null answer is a failed load, which `successful` already covers.
    let Some(accessory) =
        (unsafe { Retained::retain(accessory as *const ar_accessory_t as *mut ar_accessory_t) })
    else {
        return;
    };
    // SAFETY: the accessory was live for the call above, and it is still live here: this only reads the
    // names it reports, which are a property of the accessory and not a frame's state.
    let named = unsafe { Named::of(&accessory) };
    // SAFETY: the accessory is live for the call, and the name is copied out of ARKit's own string rather
    // than held past it.
    let name = unsafe { sys::ar_accessory_get_name(Retained::as_ptr(&accessory).cast()) };
    slot.lock()
        .expect("the accessory mailbox is only written by a callback that cannot panic")
        .push(Loaded {
            accessory: Retained::into_raw(accessory) as usize,
            named,
            // The same address a source is handed out under: `collect` finds the device this way.
            device: std::ptr::from_ref(device).cast::<c_void>() as usize,
            name,
        });
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
/// Called by ARKit with the context [`Accessories::poses`] passed, which is an `Anchors` that outlives the
/// enumeration, and with an anchor that is live for the call.
unsafe extern "C-unwind" fn collect(context: *mut c_void, anchor: &ar_accessory_anchor_t) -> bool {
    // SAFETY: the context is the `Anchors` `poses` passed, and the enumeration is synchronous with it.
    let found = unsafe { &mut *context.cast::<Anchors>() };
    // SAFETY: the anchor is live for the call, and every call below reads it rather than taking it.
    unsafe {
        let accessory = ar_accessory_anchor_t::accessory(anchor);
        // A `GCDevice` is a controller's other half, and an accessory with no source device has no
        // controller to belong to; a pose nothing can be matched to is one no caller would see.
        let Some(device) = ar_accessory_t::source_device(&accessory) else {
            return true;
        };
        // The anchor the geometry is read from: the latest one, or this frame's prediction of it. Identity,
        // the hand and the tracking state stay the latest anchor's, because none of them is what a prediction
        // changes; the place, the locations and the velocities come from the one the frame is drawn for.
        let geometry = match found.prediction {
            Some(prediction)
                if ar_accessory_anchor_t::is_tracked(anchor)
                    && ar_accessory_tracking_provider_t::predict_anchor_at_timestamp(
                        prediction.provider,
                        anchor,
                        prediction.at,
                        prediction.out,
                    ) =>
            {
                prediction.out
            }
            // No frame to predict for, or ARKit would not predict this one: the latest anchor is the honest
            // answer, and a controller a frame stale is better than one that is not there.
            _ => anchor,
        };
        let address = std::ptr::from_ref(geometry).cast::<c_void>();
        // Not the generic anchor call: ARKit's plain accessory transform is the *rendered* correction,
        // and a pose an app points with is where the accessory actually is.
        let world_from_anchor = Mat4::from_cols_array(
            &sys::ar_accessory_anchor_get_origin_from_anchor_transform_with_correction(
                address,
                ar_transform_correction_t::none.0,
            )
            .0,
        );
        let named = found
            .named
            .get(&(Retained::as_ptr(&accessory).cast::<()>() as usize))
            .copied()
            .unwrap_or_default();
        found.poses.push(AccessoryPose {
            device: Retained::as_ptr(&device).cast::<()>() as usize,
            handedness: super::handedness_of(
                ar_accessory_anchor_t::held_chirality(anchor),
                ar_accessory_t::inherent_chirality(&accessory),
            ),
            tracked: ar_accessory_anchor_t::is_tracked(anchor),
            emulated_position: super::emulated_position_of(ar_accessory_anchor_t::tracking_state(
                anchor,
            )),
            // From the latest anchor like the hand and the tracking state: a prediction moves where the
            // accessory is, not whether a hand is on it.
            held: ar_accessory_anchor_t::is_held(anchor),
            grip: if named.grip {
                location(world_from_anchor, address, ar_accessory_location_name_grip)
            } else {
                world_from_anchor
            },
            aim: if named.aim {
                location(world_from_anchor, address, ar_accessory_location_name_aim)
            } else {
                world_from_anchor
            },
            // The runtime's own estimate, which is why it is read rather than differenced between frames:
            // an accessory ARKit is tracking has a speed ARKit knows.
            linear_velocity: wxr::glam::Vec3::from_array(sys::ar_accessory_anchor_get_velocity(
                address,
            )),
            angular_velocity: wxr::glam::Vec3::from_array(
                sys::ar_accessory_anchor_get_angular_velocity(address),
            ),
        });
    }
    // Every accessory in the collection is one this session asked about, so none is skipped.
    true
}

use super::AccessoryPose;
