//! Where the world is: planes, bounds, and asking the browser for a hit.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn planes_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(reference) = self.resolved_space(space)? else {
            return Ok(());
        };
        let Some(frame) = self.frame() else {
            return Ok(());
        };
        planes::detected(&frame, &reference, &self.planes, out);
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn bounds_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(space) = self.resolved_space(space)? else {
            return Ok(());
        };
        // `boundsGeometry` is on the bounded-floor space and nowhere else, so a space that is not one is not
        // this type - which is an outline with no points rather than a failure.
        let Ok(bounds) = space.dyn_into::<XrBoundedReferenceSpace>() else {
            return Ok(());
        };
        for point in bounds.bounds_geometry().iter() {
            let Ok(point) = point.dyn_into::<DomPointReadOnly>() else {
                continue;
            };
            out.push(wxr::glam::Vec2::new(point.x() as f32, point.z() as f32));
        }
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn hit_test_source_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        // The space first, for the reason `hits` gives: a handle this session never handed out is refused
        // as such, whether or not the session was granted hit testing at all.
        let Some(base) = self.space_slot(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        // Asked of the session before the browser is: `requestHitTestSource` throws `NotSupportedError` when the
        // feature was not granted, and a session that says it does not have hit testing is a better answer than
        // an exception from inside a frame. Before the ray, too: a browser with no hit test has nothing to ask
        // along one, and "this session cannot raycast" is the truer answer than "your ray is broken".
        if !has_feature(&session, "hit-test") {
            return Err(wxr::Error::Unsupported("hit-test".into()));
        }
        // Then the ray, because a ray is only a ray in some space - and one that describes nothing is
        // refused here rather than by `new XRRay`, whose `TypeError` would arrive through the wasm frame.
        let ray = match ray {
            Some(ray) => {
                let ray = ray.unit().ok_or_else(|| {
                    wxr::Error::Rejected("a hit-test ray with no direction".into())
                })?;
                Some(hit::offset(ray)?)
            }
            None => None,
        };
        let slot = Rc::new(RefCell::new(hit::Slot::default()));
        // The space may still be a promise, and a ray out of a space that is not here is not here either - so
        // the request waits on the same promise the space does.
        let resolved = base.borrow().space.clone();
        if let Some(base) = resolved {
            hit::request(&session, &base, ray, slot.clone());
        } else if let Some(promise) = base.borrow().promise.clone() {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                {
                    hit::request(&session, &base, ray, fill);
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.hit_sources.push(slot);
        Ok(wxr::HitTestSource::new((self.hit_sources.len() - 1) as u32))
    }

    /// Let a source of hits go, which is `XRHitTestSource.cancel` and nothing more on this side.
    ///
    /// The slot stays where it is rather than coming out, because the id *is* its position in the list:
    /// pulling one out would hand every source after it somebody else's ray while their handles went on
    /// saying their own numbers. So the entry survives and the source inside it does not, which is what
    /// [`hit::Slot::cancel`] is for.
    pub(super) fn release_hit_test_source_impl(&mut self, source: wxr::HitTestSource) {
        let Some(slot) = self.hit_sources.get(source.id() as usize) else {
            return;
        };
        slot.borrow_mut().cancel();
    }
}

impl WebXrSession {
    pub(super) fn hits_impl(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        // The space before the source: a handle this session never handed out is refused as such, whether or
        // not the source is still alive.
        let Some(base) = self.resolved_space(space)? else {
            return Ok(());
        };
        let Some(slot) = self.hit_sources.get(source.id() as usize) else {
            return Ok(());
        };
        let Some(source) = slot.borrow().source() else {
            return Ok(());
        };
        let Some(frame) = self.frame() else {
            return Ok(());
        };
        hit::results(&frame, &source, &base, out);
        Ok(())
    }
}
