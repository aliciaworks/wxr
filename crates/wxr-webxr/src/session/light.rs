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
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))?;
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

impl WebXrSession {
    /// Where a probe's own space is, in one of this session's - WebXR's `XRLightProbe.probeSpace`, which only a
    /// frame's `getPose` can read. A request the browser has not answered has no space yet, and a frame outside
    /// one of its callbacks is none either, so both are `None`: the estimate's direction is in that space, and
    /// without it there is nothing to turn it into the caller's.
    pub(super) fn light_probe_pose_impl(
        &self,
        probe: wxr::LightProbe,
        space: wxr::ReferenceSpace,
    ) -> Option<wxr::Pose> {
        let base = self.resolved_space(space).ok()??;
        let probe = self
            .light_probes
            .get(probe.id() as usize)?
            .borrow()
            .probe()?;
        let frame = self.frame()?;
        let pose = frame.get_pose(&probe.probe_space(), &base)?;
        Some(transform(pose.transform()))
    }
}
