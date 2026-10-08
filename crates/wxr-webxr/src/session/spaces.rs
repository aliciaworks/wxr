//! The reference spaces this session can make.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

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
            space: None,
            promise: Some(promise.clone()),
            on_reset: None,
        }));

        // The id the handle will have, known before the space is here so that the reset handler can name it.
        let id = self.spaces.len() as u32;
        let fill = slot.clone();
        let reset = self.reset.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(value) = JsFuture::from(promise).await
                && let Ok(space) = value.dyn_into::<XrReferenceSpace>()
            {
                // The space itself says when its origin was recentered, and every pose measured in it is stale
                // afterwards - so it is passed on as the session's news, because a core event has no space to
                // arrive on.
                let handler = Closure::<dyn FnMut(Event)>::new(move |_| {
                    reset
                        .borrow_mut()
                        .push_back(wxr::Event::Reset(wxr::ReferenceSpace::new(kind, id)));
                });
                space.set_onreset(Some(handler.as_ref().unchecked_ref()));
                let mut slot = fill.borrow_mut();
                slot.space = Some(space);
                slot.promise = None;
                slot.on_reset = Some(handler);
            }
        });

        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(kind, id))
    }
}

impl WebXrSession {
    pub(super) fn offset_space_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(base) = self.spaces.get(space.id() as usize).cloned() else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let resolved = base.borrow().space.clone();
        let pending = base.borrow().promise.clone();
        let slot = Rc::new(RefCell::new(Space::default()));
        // The base may still be a promise, and an offset of a space that is not here yet is not here yet
        // either - so the same promise is waited on, which is why a slot keeps it.
        if let Some(base) = resolved {
            *slot.borrow_mut() = Space::resolved(offset_reference_space(&base, offset)?);
        } else if let Some(promise) = pending {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                    && let Ok(offset) = offset_reference_space(&base, offset)
                {
                    *fill.borrow_mut() = Space::resolved(offset);
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(
            space.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }
}
