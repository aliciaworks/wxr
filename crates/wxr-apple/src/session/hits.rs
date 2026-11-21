//! The ray each hit-test source remembers, and where it lands this frame.
//!
//! A source here is the pair the core asks for: the space its ray is measured out of, and the ray itself - or
//! the space's own aim when the app gave none, which is the default the OpenXR backend's raycaster takes too.
//! ARKit has no source object to hand back, so the table is this session's, the same shape the input sources
//! beside this file have: an id is a slot, and a released slot answers nothing.
//!
//! The cast itself is in [`crate::arkit::ArKit::cast`], because the geometry is ARKit's room and not the session's
//! spaces. What is here is the two transforms around it: from the space the ray was given in to the session's
//! origin, and from there into the space the hits were asked for in.

use wxr::glam::{Mat4, Vec3};

use super::AppleSession;

/// One source: the space its ray is in, the ray itself, and which shapes it may land on.
///
/// `ray` is `None` when the app gave none, which means the space's own `-Z` - the aim an input source points
/// along, and the same reading OpenXR's raycaster makes of it.
#[derive(Clone, Copy)]
struct Measure {
    space: wxr::ReferenceSpace,
    kinds: wxr::HitTestKinds,
    ray: Option<wxr::Ray>,
}

/// This session's hit-test sources, by the core's own id.
#[derive(Default)]
pub(super) struct Hits {
    sources: Vec<Option<Measure>>,
}

impl Hits {
    /// Keep one more source and answer with its name.
    fn add(
        &mut self,
        space: wxr::ReferenceSpace,
        kinds: wxr::HitTestKinds,
        ray: Option<wxr::Ray>,
    ) -> wxr::HitTestSource {
        let id = self.sources.len() as u32;
        self.sources.push(Some(Measure { space, kinds, ray }));
        wxr::HitTestSource::new(id)
    }

    /// Empty a slot in place, because the id *is* the position - taking it out would hand every source after
    /// it somebody else's ray.
    pub(super) fn release(&mut self, source: wxr::HitTestSource) {
        if let Some(slot) = self.sources.get_mut(source.id() as usize) {
            *slot = None;
        }
    }

    fn measure(&self, source: wxr::HitTestSource) -> Option<Measure> {
        self.sources.get(source.id() as usize).copied().flatten()
    }
}

impl AppleSession {
    /// Ask for a source of hits along a ray, or along the space's own aim when there is no ray.
    pub(super) fn hit_test_source_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        kinds: wxr::HitTestKinds,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        // The space first, as every entry that takes one does: a handle this session never handed out is
        // refused whether or not there is a room to cast at.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        // Then the room, before the ray: a session with no shape of it has nothing to ask along a ray, and
        // that is the truer answer than "your ray is broken". The kinds are part of the same question - a
        // source asking only for shapes this session never traced can never answer, and refusing it here is
        // better than handing back one that reports nothing forever.
        if !self
            .arkit
            .as_ref()
            .is_some_and(|arkit| arkit.has_hit_test_for(kinds))
        {
            return Err(wxr::Error::Unsupported("hit testing".into()));
        }
        // Then the ray, which has to name a direction: a zero, NaN or infinite one is not a place to look, and
        // casting it would report a hit at a point this backend made up.
        let ray =
            match ray {
                Some(ray) => Some(ray.unit().ok_or_else(|| {
                    wxr::Error::Rejected("a hit-test ray with no direction".into())
                })?),
                None => None,
            };
        Ok(self.hits.add(space, kinds, ray))
    }

    /// Where one source's ray landed this frame, in the space given.
    pub(super) fn hits_impl(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        // The space first, for the reason `planes` gives: a forged handle is refused the same way whether or
        // not there is a source to ask.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        // A released source, or one this session never handed out, has no ray to cast and answers with an
        // empty list - the shape the core gives a released source.
        let Some(measure) = self.hits.measure(source) else {
            return Ok(());
        };
        // A source whose space is no longer one of this session's has no ray either.
        if !self.has_space(measure.space) {
            return Ok(());
        }
        let ray = world_ray(self.world_from_space(measure.space), measure.ray);
        let Some(hit) = self
            .arkit
            .as_ref()
            .and_then(|arkit| arkit.cast(&ray, measure.kinds))
        else {
            return Ok(());
        };
        let space_from_world = wxr_render::pose_from_transform(self.space_from_world(space));
        out.push(wxr::Hit {
            pose: space_from_world.then(hit),
            // The ray was the app's, so there is no pointer to name.
            input: None,
        });
        Ok(())
    }
}

/// A source's ray in the session's own origin, which is where the room's own geometry is.
///
/// An app's ray is in the coordinates of the space it was asked in, so both halves take that space's transform
/// - and a source with no ray is the space's own aim, which is its origin and its `-Z`.
fn world_ray(world_from_space: Mat4, ray: Option<wxr::Ray>) -> wxr::Ray {
    let ray = ray.unwrap_or(wxr::Ray {
        origin: Vec3::ZERO,
        direction: Vec3::NEG_Z,
    });
    wxr::Ray {
        origin: world_from_space.transform_point3(ray.origin),
        direction: world_from_space.transform_vector3(ray.direction),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(id: u32) -> wxr::ReferenceSpace {
        wxr::ReferenceSpace::new(wxr::SpaceKind::Local, id)
    }

    #[test]
    fn a_released_source_is_empty_and_the_sources_after_it_keep_their_own_rays() {
        let mut hits = Hits::default();
        let left = hits.add(space(0), wxr::HitTestKinds::ALL, None);
        let right = hits.add(
            space(1),
            wxr::HitTestKinds::ALL,
            Some(wxr::Ray {
                origin: Vec3::X,
                direction: Vec3::NEG_Z,
            }),
        );
        hits.release(left);
        assert!(hits.measure(left).is_none());
        // The second slot is still the second slot: a release empties one entry rather than shortening the
        // list, which is what keeps an id meaning the same source for as long as the caller holds it.
        let kept = hits
            .measure(right)
            .expect("the second source is still there");
        assert_eq!(kept.space, space(1));
        assert_eq!(kept.ray.expect("it was given a ray").origin, Vec3::X);
    }

    #[test]
    fn a_ray_is_moved_out_of_its_space_and_an_aim_starts_where_that_space_is() {
        let place = wxr::glam::Mat4::from_rotation_translation(
            wxr::glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let aim = world_ray(place, None);
        assert!((aim.origin - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
        // The space looks along its own `-Z`, which a quarter turn about Y points along `-X`.
        assert!((aim.direction - Vec3::NEG_X).length() < 1e-5);
        let given = world_ray(
            place,
            Some(wxr::Ray {
                origin: Vec3::new(0.0, 0.0, -2.0),
                direction: Vec3::NEG_Z,
            }),
        );
        // Two metres in front of the space becomes two metres along its aim from where the space is.
        assert!((given.origin - Vec3::new(-1.0, 2.0, 3.0)).length() < 1e-5);
        assert!((given.direction - Vec3::NEG_X).length() < 1e-5);
    }

    #[test]
    fn a_source_this_session_never_made_has_no_ray_and_releasing_it_does_nothing() {
        let mut hits = Hits::default();
        let never = wxr::HitTestSource::new(9);
        assert!(hits.measure(never).is_none());
        hits.release(never);
        assert!(hits.measure(never).is_none());
    }
}
