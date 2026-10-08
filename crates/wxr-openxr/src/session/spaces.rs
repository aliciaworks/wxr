//! The reference spaces this session can make.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here and the
//! trait is the list of what is answered.

use super::*;

impl OpenXrSession {
    pub(super) fn space_impl(
        &mut self,
        kind: wxr::SpaceKind,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let space = self
            .session
            .create_reference_space(reference_space(kind), xr::Posef::IDENTITY)
            .map_err(|_| wxr::Error::NoSpace(kind))?;
        self.spaces.push(space);
        self.offsets.push(wxr::Pose::IDENTITY);
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
    }
}

impl OpenXrSession {
    pub(super) fn offset_space_impl(
        &mut self,
        base: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        if self.spaces.get(base.id() as usize).is_none() {
            return Err(wxr::Error::NoSpace(base.kind));
        }
        // A reference space here is a type and a pose inside it, so an offset of one is that pose composed with
        // this one - and the result is a space of the same type, which is what keeps it tracking the room.
        let inside = self.offsets[base.id() as usize].then(offset);
        let space = self
            .session
            .create_reference_space(reference_space(base.kind), posef(inside))
            .map_err(|_| wxr::Error::NoSpace(base.kind))?;
        self.spaces.push(space);
        self.offsets.push(inside);
        Ok(wxr::ReferenceSpace::new(
            base.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }
}
