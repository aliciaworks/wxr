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

impl OpenXrSession {
    /// The boundary of a bounded space, as the rectangle OpenXR knows.
    ///
    /// The two platforms describe a boundary differently and the core takes the wider of the two: WebXR gives
    /// an outline of points, OpenXR gives one width and one depth because the room it is tracking is a
    /// rectangle - so the four corners are written out here to be the outline the core's type asks for. A
    /// caller that wanted the polygon loses nothing, since a rectangle is what the runtime had.
    ///
    /// `BoundedFloor` is the only kind that has one - it is the space `STAGE` became, whose whole definition
    /// includes the boundary - so any other kind is an outline with no points, and a stage the user has not
    /// walked a room for is the same. Neither is an error: a floor with no wall drawn around it is a floor.
    pub(super) fn bounds_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        if space.kind != wxr::SpaceKind::BoundedFloor {
            return Ok(());
        }
        // A space this session never handed out has no boundary to ask about, and asking the runtime about one
        // it does not know is a fault rather than an empty answer.
        if self.spaces.get(space.id() as usize).is_none() {
            return Ok(());
        }

        // `xrGetReferenceSpaceBoundsRect`, which answers `SPACE_BOUNDS_UNAVAILABLE` as a `None` - the runtime
        // is willing and has nothing, which is the same shape as the other two cases above.
        let Some(bounds) = self
            .session
            .reference_space_bounds_rect(xr::ReferenceSpaceType::STAGE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?
        else {
            return Ok(());
        };

        // The runtime's rectangle is in `STAGE`, which is the base kind an offset space is made from. The
        // core asks for the boundary in the space it was handed, so the corners are taken back through the
        // pose that made that space - otherwise an app that offset its floor would get the room's rectangle
        // drawn as if the offset had never happened.
        let inside = self
            .offsets
            .get(space.id() as usize)
            .copied()
            .unwrap_or_default();
        out.extend(corners(inside, bounds.width, bounds.height));
        Ok(())
    }
}

/// The four corners of a `width` by `height` rectangle, in `space`.
///
/// The runtime says `STAGE`'s boundary by its size, centred on `STAGE`'s origin; `space` is an offset of
/// that origin, so a corner is `space` taken back to `STAGE` before it is read. Walked anticlockwise seen
/// from above, which is the winding WebXR hands over for the same room.
fn corners(space: wxr::Pose, width: f32, height: f32) -> [wxr::glam::Vec2; 4] {
    let back = space.transform().inverse();
    let half = (width * 0.5, height * 0.5);
    [
        (-half.0, -half.1),
        (half.0, -half.1),
        (half.0, half.1),
        (-half.0, half.1),
    ]
    .map(|(x, z)| {
        let point = back.transform_point3(wxr::glam::Vec3::new(x, 0.0, z));
        wxr::glam::Vec2::new(point.x, point.z)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wxr::glam::{Quat, Vec2, Vec3};

    #[test]
    fn a_plain_floor_keeps_the_runtime_corners() {
        // The common case is the base space itself, and then there is nothing to undo: a four by two room
        // is its four corners and no arithmetic worth getting wrong.
        let got = corners(wxr::Pose::IDENTITY, 4.0, 2.0);
        assert_eq!(
            got,
            [
                Vec2::new(-2.0, -1.0),
                Vec2::new(2.0, -1.0),
                Vec2::new(2.0, 1.0),
                Vec2::new(-2.0, 1.0),
            ]
        );
    }

    #[test]
    fn an_offset_floor_moves_the_boundary_with_it() {
        // A space a metre along `x` is where the wearer put it; the room has not moved, so the room's
        // corner a metre behind that origin is at -2 and not at -1.
        let space = wxr::Pose {
            position: Vec3::new(1.0, 0.0, 0.0),
            orientation: Quat::IDENTITY,
        };
        let got = corners(space, 2.0, 2.0);
        assert!(got[0].abs_diff_eq(Vec2::new(-2.0, -1.0), 1e-6));
        assert!(got[1].abs_diff_eq(Vec2::new(0.0, -1.0), 1e-6));
    }
}
