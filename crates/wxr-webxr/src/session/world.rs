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
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
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
        let Some(space) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
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
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        // Asked of the session before the browser is: `requestHitTestSource` throws `NotSupportedError` when the
        // feature was not granted, and a session that says it does not have hit testing is a better answer than
        // an exception from inside a frame.
        if !has_feature(&session, "hit-test") {
            return Err(wxr::Error::Unsupported("hit-test".into()));
        }
        let Some(base) = self.spaces.get(space.id() as usize).cloned() else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let slot = Rc::new(RefCell::new(hit::Slot::default()));
        // The space may still be a promise, and a ray out of a space that is not here is not here either - so
        // the request waits on the same promise the space does.
        let resolved = base.borrow().space.clone();
        if let Some(base) = resolved {
            hit::request(&session, &base, slot.clone());
        } else if let Some(promise) = base.borrow().promise.clone() {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                {
                    hit::request(&session, &base, fill);
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.hit_sources.push(slot);
        Ok(wxr::HitTestSource::new((self.hit_sources.len() - 1) as u32))
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
        let Some(slot) = self.hit_sources.get(source.id() as usize) else {
            return Ok(());
        };
        let Some(source) = slot.borrow().source() else {
            return Ok(());
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        hit::results(&frame, &source, &base, out);
        Ok(())
    }
}
