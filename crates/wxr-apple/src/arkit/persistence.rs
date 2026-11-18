//! Names an anchor keeps after the session ends: ARKit's persisted world anchors.
//!
//! A world anchor ARKit has accepted outlives the app - "persisted across device restarts until explicitly
//! removed", as the header says of `ar_world_tracking_provider_add_anchor` - and the identifier it was given
//! is what finds it again. Two calls are the whole of that API: `copy_all_world_anchors` hands back every
//! anchor the provider knows, persisted ones from earlier runs included, and `remove_anchor_with_identifier`
//! forgets one by name. Neither needs an `ARWorldMap`, which is the binding this backend does not have.
//!
//! # Why a mailbox sits between those calls and the frame
//!
//! Copying is asynchronous, and its completion runs on a queue of ARKit's choosing - unlike the light
//! provider's handler, the function variant takes no queue to run on. A callback on somebody else's thread
//! must not touch this session, so it touches a leaked [`Mutex`] instead: it retains each anchor it finds and
//! records the address, and the session takes that list apart on the thread that asks. A retained anchor is
//! an object ARC keeps alive across the hand-off; an address is the only way to move one between threads
//! without claiming the object is `Send`, and only this module ever turns one back into a `Retained`.
//!
//! # What a name is answered with
//!
//! Reading is the same pull shape every other ARKit answer has. [`wxr::Session::restore_anchor`] hands back a
//! handle now and starts one copy; when the completion lands, the anchor whose identifier was asked for goes
//! into that slot and `anchor_pose` starts answering. A name the provider does not know leaves the slot empty
//! for good, which is the core's answer for a name that finds nothing. [`wxr::Session::persistent_anchor_ids`]
//! asks the same copy for the names alone, and keeps the last completion's list whether or not a handle asked
//! for one.
//!
//! Erasing needs no lookup at all: the call takes the identifier, so forgetting a name is one request rather
//! than the round trip reading is. The core's answer is `Ok` once the request is made, and a name the provider
//! does not have is not an error to report - which is why the table is cleared before the call rather than
//! from its completion.

use std::ffi::c_void;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_error_t, ar_world_anchor_t, ar_world_anchors_t, ar_world_tracking_provider_t,
};

use super::ArKit;

/// One name and the address of one retained anchor: what a completion hands the session's thread.
type Found = ([u8; 16], usize);

/// What one copy completion has left for the session to read.
#[derive(Default)]
pub(super) struct Listing {
    /// Every anchor the last completion found, as the identifier it was given and the address of a `+1`.
    /// The address is not a reference: it is how the `+1` crosses a thread, and the session's own thread is
    /// the only place it is turned back into a [`Retained`].
    found: Vec<Found>,
    /// Every name the last completion found, whether or not a handle asked for it: the answer
    /// [`wxr::Session::persistent_anchor_ids`] gives, kept apart from `found` because `found` is taken by
    /// [`ArKit::drain`] and these have to outlive that.
    names: Vec<[u8; 16]>,
    /// How many completions have arrived. A slot asked for before a copy was requested is answered by the
    /// next completion whether or not it finds the anchor, which is what keeps a missing name from waiting
    /// forever.
    completed: u64,
    /// Whether a copy is out. Requests are one at a time because a completion answers every ask that was
    /// already waiting - its results are read when it arrives, not when it was asked for.
    in_flight: bool,
}

/// A name asked for from an earlier run, and the completion that will decide it.
pub(super) struct Waiting {
    pub(super) id: wxr::AnchorId,
    /// The completion count that decides this slot: the first one that reaches it either finds the anchor or
    /// proves the provider does not have it.
    after: u64,
}

impl ArKit {
    /// The identifier ARKit gave a world anchor, which is the name that outlives the session.
    pub(super) fn identifier(anchor: &ar_world_anchor_t) -> wxr::AnchorId {
        let mut bytes = [0u8; 16];
        // SAFETY: the anchor is live, and ARKit fills the UUID it made it with.
        unsafe { ar_world_anchor_t::identifier(anchor, &mut bytes) };
        wxr::AnchorId::from_bytes(bytes)
    }

    /// The next slot a handle may name. NEVER reused: a caller holding an id has to keep holding the same one.
    pub(super) fn next_slot(&mut self) -> u32 {
        let slot = self.next_anchor;
        self.next_anchor += 1;
        slot
    }

    /// Take whatever a completion has found and put it in the slot that asked.
    ///
    /// Called before every anchor answer rather than once a frame: it is a lock and a list that is empty on
    /// most calls, and doing it here is what makes a restored handle start working on the frame the runtime
    /// answers instead of on a frame the caller has no reason to think about.
    pub(super) fn drain(&mut self) {
        let (found, completed) = {
            // A poisoned slot means something panicked while holding it, and the only writers are handlers
            // that cannot panic and this function - so a frame is the wrong place to find that out.
            let mut listing = self
                .listing
                .lock()
                .expect("the anchor mailbox is only written by handlers that cannot panic");
            (std::mem::take(&mut listing.found), listing.completed)
        };
        for (bytes, address) in found {
            let id = wxr::AnchorId::from_bytes(bytes);
            // SAFETY: the completion took a `+1` on this object and this is the only thread that reads the
            // address; the address is one the mailbox handed over exactly once.
            let Some(anchor) = (unsafe { Retained::from_raw(address as *mut ar_world_anchor_t) })
            else {
                continue;
            };
            // Every handle that asked for this name gets one: two restores of the same name are two handles
            // on one object, and one completion has to satisfy both or the second would never answer.
            let mut slots: Vec<u32> = self
                .waiting
                .iter()
                .filter(|(_, waiting)| waiting.id == id)
                .map(|(slot, _)| *slot)
                .collect();
            slots.sort_unstable();
            let Some((first, rest)) = slots.split_first() else {
                // Nobody asked for this one. Dropping the `+1` gives it back to ARKit, which still tracks
                // it - a place this session did not ask about is not one it should be holding.
                continue;
            };
            self.anchors.insert(*first, anchor);
            for slot in rest {
                // SAFETY: the object is live and now held by the map, so a second `+1` is a second handle on
                // the same place; the provider still keeps it, and the map keeps both alive.
                let second =
                    unsafe { Retained::retain(Retained::as_ptr(&self.anchors[first]).cast_mut()) };
                if let Some(second) = second {
                    self.anchors.insert(*slot, second);
                }
            }
            for slot in &slots {
                self.waiting.remove(slot);
            }
        }
        // A slot whose deciding completion has come and gone was not in it: the provider does not have that
        // name, and the handle it was given stays empty.
        self.waiting.retain(|_, waiting| waiting.after > completed);
    }

    /// Every name the provider is keeping, starting a copy when none is out.
    ///
    /// Empty until a copy has answered, a frame or two after the first call - the same pull shape
    /// [`ArKit::restore_anchor`] has. `false` when there is no world tracking and so no provider to ask.
    pub fn persistent_ids(&mut self, out: &mut Vec<wxr::AnchorId>) -> bool {
        out.clear();
        if self.world.is_none() {
            return false;
        }
        // Take what a completion has handed over first, so the frame after it answers is the first that has
        // the name rather than the one after that.
        self.drain();
        {
            let listing = self
                .listing
                .lock()
                .expect("the anchor mailbox is only written by handlers that cannot panic");
            out.extend(listing.names.iter().copied().map(wxr::AnchorId::from_bytes));
        }
        self.ask_for_all();
        true
    }

    /// A handle for a place ARKit stored under `id`, or `None` when there is no world to look in.
    ///
    /// A name the provider is already tracking - one the app made this session, or one an earlier lookup
    /// found - is answered at once with a second handle on the same object. Anything else becomes a slot that
    /// `anchor_pose` answers `None` for until a copy completes, which is the same shape the creation path
    /// has: the handle arrives before the place does.
    pub fn restore_anchor(&mut self, id: wxr::AnchorId) -> Option<wxr::Anchor> {
        // A place nobody is keeping cannot be found, which is the same `Unsupported` a runtime without
        // anchors answers with - and the answer does not depend on the name.
        self.world.as_ref()?;
        // Asked for by name, so the name is the app's and the place has to outlive the handle.
        self.named.insert(id);
        self.drain();
        let held = self
            .anchors
            .iter()
            .find(|(_, anchor)| Self::identifier(anchor) == id)
            .map(|(slot, _)| *slot);
        if let Some(slot) = held {
            // SAFETY: the anchor is live and tracked, so a second `+1` is a second handle on it. The
            // provider still keeps it; releasing either handle leaves the other working.
            let anchor =
                unsafe { Retained::retain(Retained::as_ptr(&self.anchors[&slot]).cast_mut()) }?;
            let slot = self.next_slot();
            self.anchors.insert(slot, anchor);
            return Some(wxr::Anchor::new(slot));
        }
        let slot = self.next_slot();
        // The next completion decides this slot: its results are read when it arrives, which is after this
        // ask whether the copy was already out or starts here.
        let after = {
            let listing = self
                .listing
                .lock()
                .expect("the anchor mailbox is only written by handlers that cannot panic");
            listing.completed + 1
        };
        self.waiting.insert(slot, Waiting { id, after });
        self.ask_for_all();
        Some(wxr::Anchor::new(slot))
    }

    /// Forget a name, which ARKit takes by identifier rather than by object.
    ///
    /// `false` when there is no world provider, where a name has nothing to mean. A name that is also one of
    /// this session's handles drops that handle's object too: ARKit has been told to forget the place, and a
    /// handle left in the table would keep answering for it.
    pub fn delete_persistent_anchor(&mut self, id: wxr::AnchorId) -> bool {
        self.drain();
        self.anchors
            .retain(|_, anchor| Self::identifier(anchor) != id);
        // A slot still waiting for the name asked for a place that will not be there.
        self.waiting.retain(|_, waiting| waiting.id != id);
        // The app asked for the name to be gone, so the next release of a handle to it is not a reason to
        // keep it.
        self.named.remove(&id);
        let Some(world) = self.world.as_ref() else {
            return false;
        };
        let mut bytes = *id.as_bytes();
        // SAFETY: the provider is live, the identifier outlives the call, and the completion only logs. A
        // name the provider does not have completes with `false`, which is the no-op the core asks for.
        unsafe {
            ar_world_tracking_provider_t::remove_anchor_with_identifier_f(
                &world.provider,
                &mut bytes,
                std::ptr::null_mut(),
                erased,
            );
        }
        true
    }

    /// Ask for every anchor the provider knows, unless an ask is already out.
    fn ask_for_all(&mut self) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        {
            let mut listing = self
                .listing
                .lock()
                .expect("the anchor mailbox is only written by handlers that cannot panic");
            if listing.in_flight {
                return;
            }
            // Set before the call and not after it: the completion can arrive before it returns, and a
            // second ask made from that callback would be a second request for the same list.
            listing.in_flight = true;
        }
        // SAFETY: the provider is live, the mailbox is leaked and so outlives any completion, and the
        // completion touches nothing but that mailbox.
        unsafe {
            ar_world_tracking_provider_t::copy_all_world_anchors_f(
                &world.provider,
                self.listing as *const Mutex<Listing> as *mut c_void,
                Some(listed),
            );
        }
    }
}

/// The completion `ask_for_all` hands ARKit: retain what it found, and count the answer.
///
/// # Safety
///
/// Called by ARKit with the context [`ArKit::ask_for_all`] passed, which is a leaked slot, and with a
/// collection that is live for the call.
unsafe extern "C-unwind" fn listed(context: *mut c_void, anchors: Option<&ar_world_anchors_t>) {
    // SAFETY: the context is the address `ask_for_all` leaked, which lives as long as the process does.
    let slot = unsafe { &*context.cast::<Mutex<Listing>>() };
    let mut found: Vec<Found> = Vec::new();
    if let Some(anchors) = anchors {
        // SAFETY: the collection is live for the call, and the enumerator only reads the anchors it is
        // handed. Retaining happens inside `hold`, on this thread, before the addresses are stored.
        unsafe {
            ar_world_anchors_t::enumerate_anchors_f(
                anchors,
                std::ptr::from_mut(&mut found).cast::<c_void>(),
                hold,
            );
        }
    }
    let mut listing = slot
        .lock()
        .expect("the anchor mailbox is only written by handlers that cannot panic");
    // The last copy is the list of names: replaced rather than added to, so a name ARKit no longer has stops
    // being offered the next time a list is asked for.
    listing.names = found.iter().map(|(bytes, _)| *bytes).collect();
    listing.found.extend(found);
    listing.completed += 1;
    listing.in_flight = false;
}

/// The enumerator `listed` hands ARKit: one anchor, retained, as its identifier and its address.
///
/// # Safety
///
/// `context` must be the `*mut Vec<Found>` the caller passed, and `anchor` a live world anchor.
unsafe extern "C-unwind" fn hold(context: *mut c_void, anchor: &ar_world_anchor_t) -> bool {
    let found = context.cast::<Vec<Found>>();
    let id = ArKit::identifier(anchor);
    // SAFETY: the anchor is live and ours only for the call, so the `+1` is what lets it outlive one. The
    // object is not `Send` and is never treated as one: what leaves this function is an address.
    unsafe {
        if let Some(held) = Retained::retain(std::ptr::from_ref(anchor).cast_mut()) {
            (*found).push((*id.as_bytes(), Retained::into_raw(held) as usize));
        }
    }
    true
}

/// The completion `delete_persistent_anchor` hands ARKit: a name was forgotten, or it was not there.
///
/// The table was already cleared before the request was made, so the only thing left to hear is a failure
/// worth a line: a name the provider does not have is a no-op, and a request that fails for another reason
/// leaves the place persisted - which the app finds out about the next time it restores the name, not here.
///
/// # Safety
///
/// Called by ARKit with a null context, which is why nothing is dereferenced, and an anchor that is live for
/// the call.
unsafe extern "C-unwind" fn erased(
    _context: *mut c_void,
    _anchor: &ar_world_anchor_t,
    success: bool,
    error: Option<&ar_error_t>,
) {
    if !success {
        log::debug!("wxr-apple: a persistent anchor was not erased: {error:?}");
    }
}
