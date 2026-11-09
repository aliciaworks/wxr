//! What the browser's lighting probe says.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn light_probe_impl(&mut self) -> Result<wxr::LightProbe, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        // `requestLightProbe` throws `NotSupportedError` when the feature was not granted and
        // `InvalidStateError` once the session has ended, rather than rejecting. The feature check covers the
        // usual case; the caught call covers a session that ended between the check and the request.
        if !has_feature(&session, "light-estimation") {
            return Err(wxr::Error::Unsupported("light-estimation".into()));
        }
        let slot = Rc::new(RefCell::new(crate::light::Slot::default()));
        crate::light::request(&session, slot.clone())
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))?;
        self.light_probes.push(slot);
        Ok(wxr::LightProbe::new((self.light_probes.len() - 1) as u32))
    }
}

impl WebXrSession {
    pub(super) fn light_impl(
        &mut self,
        probe: wxr::LightProbe,
        out: &mut wxr::LightEstimate,
    ) -> Result<(), wxr::Error> {
        *out = wxr::LightEstimate::default();
        let Some(slot) = self.light_probes.get(probe.id() as usize) else {
            return Ok(());
        };
        let Some(probe) = slot.borrow().probe() else {
            return Ok(());
        };
        let Some(frame) = self.frame() else {
            return Ok(());
        };
        crate::light::estimate(&frame, &probe, out);
        Ok(())
    }
}
