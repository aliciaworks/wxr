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
        // For the same reason as a hit-test source: `requestLightProbe` throws `NotSupportedError` rather than
        // rejecting when the feature is not there.
        if !has_feature(&session, "light-estimation") {
            return Err(wxr::Error::Unsupported("light-estimation".into()));
        }
        let slot = Rc::new(RefCell::new(crate::light::Slot::default()));
        crate::light::request(&session, slot.clone());
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
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        crate::light::estimate(&frame, &probe, out);
        Ok(())
    }
}
