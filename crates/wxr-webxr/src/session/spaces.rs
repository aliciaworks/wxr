//! The reference spaces this session can make.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

/// The handler that passes a space's own recentering on as the session's news.
///
/// The specification requires a `reset` to be dispatched on every offset space of the space that reset,
/// so each handle needs its own handler - a core event has no space to arrive on, and a handle whose
/// slot never holds this closure is a handle that never hears anything. Keeping it in the slot is the
/// half the compiler does not check: a closure the browser holds and this drops stops being called.
fn reset_handler(
    reset: Rc<RefCell<VecDeque<wxr::Event>>>,
    kind: wxr::SpaceKind,
    id: u32,
) -> Closure<dyn FnMut(Event)> {
    Closure::<dyn FnMut(Event)>::new(move |_| {
        reset
            .borrow_mut()
            .push_back(wxr::Event::Reset(wxr::ReferenceSpace::new(kind, id)));
    })
}

impl WebXrSession {
    pub(super) fn space_impl(
        &mut self,
        kind: wxr::SpaceKind,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let promise: js_sys::Promise = session
            .request_reference_space(reference_space_type(kind))
            .unchecked_into();
        let slot = Rc::new(RefCell::new(Space {
            kind,
            space: None,
            promise: Some(promise.clone()),
            on_reset: None,
        }));

        // The id the handle will have, known before the space is here so that the reset handler can name it.
        let id = self.spaces.len() as u32;
        let fill = slot.clone();
        let reset = self.reset.clone();
        wasm_bindgen_futures::spawn_local(async move {
            // A rejected promise and an answer that is not a space are the same thing here: the browser has
            // settled the request without a space, and it will never settle it again. The promise is cleared
            // either way, because an offset of a space that is still *waiting* waits on the same promise - and
            // an offset of a space that was refused would then hand back a handle that never resolves instead
            // of the `NoSpace` the caller can act on.
            let space = match JsFuture::from(promise).await {
                Ok(value) => value.dyn_into::<XrReferenceSpace>().ok(),
                Err(_) => None,
            };
            let mut slot = fill.borrow_mut();
            slot.promise = None;
            let Some(space) = space else {
                return;
            };
            // The space itself says when its origin was recentered, and every pose measured in it is stale
            // afterwards - so it is passed on as the session's news, because a core event has no space to
            // arrive on.
            let handler = reset_handler(reset, kind, id);
            space.set_onreset(Some(handler.as_ref().unchecked_ref()));
            slot.space = Some(space);
            slot.on_reset = Some(handler);
        });

        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(kind, id))
    }
}

impl WebXrSession {
    /// The slot a handle names, if this session made that id of that kind.
    ///
    /// The id is an index and the kind is public on the handle, so both have to agree: an id past the list
    /// belongs to some other session, and a kind a caller changed is not the space this session made.
    pub(super) fn space_slot(&self, space: wxr::ReferenceSpace) -> Option<Rc<RefCell<Space>>> {
        let slot = self.spaces.get(space.id() as usize)?;
        (slot.borrow().kind == space.kind).then(|| slot.clone())
    }

    /// The browser's space behind a handle, once the browser has answered for it.
    ///
    /// `Err(NoSpace)` is a handle this session never handed out - an id past the list, or one whose public
    /// kind was changed - and `Ok(None)` is a space this session did make that the browser has not settled
    /// yet. The caller decides because the two callers differ: a creator has nothing to build from a promise
    /// and says so, while a reader asked before the first frame has no eyes to report either way.
    pub(super) fn resolved_space(
        &self,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<XrReferenceSpace>, wxr::Error> {
        let Some(slot) = self.space_slot(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        Ok(slot.borrow().space.clone())
    }

    pub(super) fn offset_space_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(base) = self.space_slot(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let resolved = base.borrow().space.clone();
        let pending = base.borrow().promise.clone();
        let slot = Rc::new(RefCell::new(Space {
            kind: space.kind,
            space: None,
            promise: None,
            on_reset: None,
        }));
        // The id the handle will have, known before the space is here so that the reset handler can name it.
        let id = self.spaces.len() as u32;
        let reset = self.reset.clone();
        // The base may still be a promise, and an offset of a space that is not here yet is not here yet
        // either - so the same promise is waited on, which is why a slot keeps it.
        if let Some(base) = resolved {
            let offset = offset_reference_space(&base, offset)?;
            let handler = reset_handler(reset, space.kind, id);
            offset.set_onreset(Some(handler.as_ref().unchecked_ref()));
            *slot.borrow_mut() = Space {
                kind: space.kind,
                space: Some(offset),
                promise: None,
                on_reset: Some(handler),
            };
        } else if let Some(promise) = pending {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                    && let Ok(offset) = offset_reference_space(&base, offset)
                {
                    let handler = reset_handler(reset, space.kind, id);
                    offset.set_onreset(Some(handler.as_ref().unchecked_ref()));
                    *fill.borrow_mut() = Space {
                        kind: space.kind,
                        space: Some(offset),
                        promise: None,
                        on_reset: Some(handler),
                    };
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(space.kind, id))
    }
}
