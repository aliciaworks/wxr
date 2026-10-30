//! The surfaces the runtime has found, which is `XR_EXT_plane_detection` driven by hand.
//!
//! The typed crate loads this extension's function pointers and wraps none of them, so the state machine is
//! this file's: a detector is made once, a pass is begun, polled until it is done, read into a buffer this
//! backend owns, and begun again. That is the spec's own example, and the reason it is a state machine at
//! all is that looking for planes takes time - which is why one call to [`wxr::Session::planes`] is one
//! *step* and an empty list is a pass that has not finished yet.
//!
//! What is not here is the polygon. `XR_EXT_plane_detection` will hand back each plane's outer contour and
//! its holes, and [`wxr::Plane`] is a bounding box with an extent - which is the shape all three backends
//! have, and deliberately not the outline the other two have and this one would. Asking the runtime for
//! contours it would then throw away is why `PlaneDetectorFlagsEXT` is empty rather than `ENABLE_CONTOUR`.

use super::*;

/// How many planes one pass will ask the runtime for.
///
/// Not "as many as there are": `max_planes` must be more than zero or the runtime answers
/// `XR_ERROR_VALIDATION_FAILURE`, and the core has no notion of a limit to ask about. Sixty-four is past
/// what a room has surfaces in, and the read below takes whatever the pass actually produced - so this
/// caps the pass rather than the answer.
const MAX_PLANES: u32 = 64;

/// The edge of the volume planes are looked for in, in metres.
///
/// `XR_EXT_plane_detection` finds planes inside a box fixed in a space, and an app that does not say which
/// box means "the room" - which is a few metres across on any hardware that can see one. This is the box,
/// centred on this backend's own `LOCAL`: 32 metres, so a wall at the far side of a large room still
/// counts.
const DETECTION_VOLUME: f32 = 32.0;

/// One plane detector, and whether a pass is running on it.
pub(super) struct PlaneDetector {
    /// The session, kept for its `Arc` rather than for anything it is used to call: the detector and the
    /// space below are the session's children, and a child destroyed after its parent is a call into a
    /// handle the runtime has already freed. This is what makes [`Drop for PlaneDetector`] safe.
    #[allow(dead_code, reason = "held for its lifetime, not for a call")]
    session: xr::Session<xr::Vulkan>,
    table: xr::raw::PlaneDetectionEXT,
    /// The detector itself. Made in [`Self::new`] and kept for the session, because a detector whose pass is
    /// in flight is a detector that is not finished, and this backend is the one that has to notice.
    detector: openxr_sys::PlaneDetectorEXT,
    /// The space the detection volume is fixed in, which is this backend's own `LOCAL` and not one the app
    /// made - because a volume that moved with the app's choice of space would be a volume that moved.
    local: xr::Space,
    /// Whether a pass has been begun and has not been read yet.
    running: bool,
    /// Whether `XR_PLANE_DETECTION_STATE_FATAL_EXT` has been seen, which the spec says must not be retried.
    fatal: bool,
}

impl PlaneDetector {
    /// Make the one detector this session will have, or say the runtime cannot.
    pub(super) fn new(session: &xr::Session<xr::Vulkan>, enabled: bool) -> Option<Self> {
        if !enabled {
            log::info!("wxr-openxr: no planes: this runtime has no XR_EXT_plane_detection");
            return None;
        }
        let Some(table) = session.instance().exts().ext_plane_detection.as_ref() else {
            log::info!("wxr-openxr: no planes: the extension was enabled but did not load");
            return None;
        };
        let local = match session
            .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
        {
            Ok(space) => space,
            Err(error) => {
                log::warn!("wxr-openxr: no planes: making the LOCAL space to look in: {error:?}");
                return None;
            }
        };
        let create_info = openxr_sys::PlaneDetectorCreateInfoEXT {
            ty: openxr_sys::StructureType::PLANE_DETECTOR_CREATE_INFO_EXT,
            next: std::ptr::null(),
            // No contours: the core's plane is a bounding box, and this is where the decision to not ask
            // for the outline is recorded - see the module.
            flags: openxr_sys::PlaneDetectorFlagsEXT::EMPTY,
        };
        let mut detector = null_detector();
        // SAFETY: every pointer here points at a value that outlives the call, and the session and the
        // table are the ones the runtime gave out for this instance.
        let result =
            unsafe { (table.create_plane_detector)(session.as_raw(), &create_info, &mut detector) };
        if result != openxr_sys::Result::SUCCESS {
            log::warn!("wxr-openxr: no planes: making a detector: {result:?}");
            return None;
        }
        Some(Self {
            session: session.clone(),
            table: *table,
            detector,
            local,
            running: false,
            fatal: false,
        })
    }
}

impl OpenXrSession {
    /// One step of the detection loop, and whatever the last finished pass found.
    ///
    /// A pass begun here will not be finished here, so the call that begins one answers empty; the call that
    /// finds it done answers its planes and begins the next pass before returning, so a pass that completes
    /// in one step does not leave every other call empty.
    pub(super) fn planes_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(detector) = self.planes.as_mut() else {
            return Ok(());
        };
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        // `FATAL` is terminal - the spec says the detector must not be used again - and a pass that ended in
        // it can leave `running` true or hand back `ERROR` later. Both would otherwise begin a second pass
        // on a detector the runtime has already given up on.
        if detector.fatal {
            return Ok(());
        }
        let time = self.predicted;

        if !detector.running {
            detector.begin(time);
            return Ok(());
        }

        // SAFETY: the detector is live, and the state is an out-parameter the runtime fills.
        let mut state = openxr_sys::PlaneDetectionStateEXT::NONE;
        let result =
            unsafe { (detector.table.get_plane_detection_state)(detector.detector, &mut state) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: asking for the plane detection state: {result:?}");
            detector.running = false;
            return Ok(());
        }
        if state != openxr_sys::PlaneDetectionStateEXT::DONE {
            // Still looking. `ERROR` is a pass that may be tried again and `FATAL` one that must not be,
            // which is the whole difference between restarting here and stopping.
            if state == openxr_sys::PlaneDetectionStateEXT::FATAL && !detector.fatal {
                detector.fatal = true;
                log::warn!("wxr-openxr: plane detection gave up; there will be no more planes");
            }
            if state == openxr_sys::PlaneDetectionStateEXT::ERROR {
                detector.running = false;
            }
            return Ok(());
        }

        detector.read(reference.as_raw(), time, out);
        // Start the next pass here rather than on the next call. Doing it on the next call leaves the
        // caller's list empty every other frame, and a room that blinks is worse than one that is a frame
        // stale. The pass just read is discarded by this begin, but its planes are already in `out`.
        detector.running = false;
        detector.begin(time);
        Ok(())
    }
}

impl PlaneDetector {
    /// Begin a pass over the volume this detector was made with.
    fn begin(&mut self, time: xr::Time) {
        let info = openxr_sys::PlaneDetectorBeginInfoEXT {
            ty: openxr_sys::StructureType::PLANE_DETECTOR_BEGIN_INFO_EXT,
            next: std::ptr::null(),
            base_space: self.local.as_raw(),
            time,
            // No orientation filter and no semantic filter: the runtime's answer is the room, and which way
            // a surface lies is a field on it rather than a reason to not report it.
            orientation_count: 0,
            orientations: std::ptr::null(),
            semantic_type_count: 0,
            semantic_types: std::ptr::null(),
            max_planes: MAX_PLANES,
            min_area: 0.0,
            bounding_box_pose: xr::Posef::IDENTITY,
            bounding_box_extent: openxr_sys::Extent3DfEXT {
                width: DETECTION_VOLUME,
                height: DETECTION_VOLUME,
                depth: DETECTION_VOLUME,
            },
        };
        // SAFETY: the detector is live and `info` outlives the call.
        let result = unsafe { (self.table.begin_plane_detection)(self.detector, &info) };
        if result == openxr_sys::Result::SUCCESS {
            self.running = true;
        } else {
            log::debug!("wxr-openxr: beginning a plane detection pass: {result:?}");
            self.running = false;
        }
    }

    /// Take the finished pass's planes out, in whichever space the caller asked for.
    fn read(&self, base_space: openxr_sys::Space, time: xr::Time, out: &mut Vec<wxr::Plane>) {
        let info = openxr_sys::PlaneDetectorGetInfoEXT {
            ty: openxr_sys::StructureType::PLANE_DETECTOR_GET_INFO_EXT,
            next: std::ptr::null(),
            // The space the pass was begun in is the space the *volume* is fixed in; this one is only where
            // the answers come back expressed. The spec is explicit that the two are separate: "the plane
            // pose will be relative to this XrSpace at time".
            base_space,
            time,
        };

        // The two-call idiom, which is the only way OpenXR lets an app find out how many there are.
        let mut locations = openxr_sys::PlaneDetectorLocationsEXT {
            ty: openxr_sys::StructureType::PLANE_DETECTOR_LOCATIONS_EXT,
            next: std::ptr::null_mut(),
            plane_location_capacity_input: 0,
            plane_location_count_output: 0,
            plane_locations: std::ptr::null_mut(),
        };
        // SAFETY: `locations` is an out-struct with a null element array and a zero capacity, which is what
        // the runtime is told to expect when it is only being asked for a count.
        let result =
            unsafe { (self.table.get_plane_detections)(self.detector, &info, &mut locations) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: asking how many planes there are: {result:?}");
            return;
        }
        let count = locations.plane_location_count_output as usize;
        if count == 0 {
            return;
        }

        // Every element is an output structure with a `type` and a `next`, so each one is built rather than
        // left as whatever memory it is - which is a rule the runtime checks and a crash when it does not.
        let mut buffer: Vec<openxr_sys::PlaneDetectorLocationEXT> =
            (0..count).map(|_| empty_location()).collect();
        locations.plane_location_capacity_input = count as u32;
        locations.plane_locations = buffer.as_mut_ptr();
        // SAFETY: the buffer holds exactly `plane_location_capacity_input` elements, each of which has the
        // type and the null chain the runtime is told to expect, and it outlives the call.
        let result =
            unsafe { (self.table.get_plane_detections)(self.detector, &info, &mut locations) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: reading the plane detections: {result:?}");
            return;
        }

        out.reserve(count);
        for location in buffer
            .iter()
            .take(locations.plane_location_count_output as usize)
        {
            // "If none of the bits are set, no other fields in this structure should be considered to be
            // valid or meaningful." A plane whose place the runtime will not stand behind is not a plane to
            // report, and half of one is worse than none.
            if !is_located(location.location_flags) {
                continue;
            }
            out.push(wxr::Plane {
                // The runtime's own id for the surface, which stays with it while the detector is alive and
                // is the thing a caller compares to tell one plane from another.
                id: location.plane_id as u32,
                pose: crate::convert::surface(crate::convert::pose(location.pose)),
                // OpenXR measures the extent along the pose's X and Y and the core along its X and Z - so
                // the two numbers carry over unchanged, which is what `convert::surface`'s rotation is for.
                extent: wxr::glam::Vec2::new(location.extents.width, location.extents.height),
                orientation: orientation(location.orientation),
            });
        }
    }
}

/// How OpenXR says a surface lies, in the core's terms.
///
/// OpenXR has four and the core has three, and the one OpenXR has that the core does not is the honest
/// answer to a surface the runtime found and will not classify: `ARBITRARY` is not "not horizontal", it is
/// "did not say", which is what [`wxr::PlaneOrientation::Unknown`] is for.
fn orientation(found: openxr_sys::PlaneDetectorOrientationEXT) -> wxr::PlaneOrientation {
    match found {
        o if o == openxr_sys::PlaneDetectorOrientationEXT::HORIZONTAL_UPWARD => {
            wxr::PlaneOrientation::Horizontal
        }
        o if o == openxr_sys::PlaneDetectorOrientationEXT::HORIZONTAL_DOWNWARD => {
            wxr::PlaneOrientation::Horizontal
        }
        o if o == openxr_sys::PlaneDetectorOrientationEXT::VERTICAL => {
            wxr::PlaneOrientation::Vertical
        }
        _ => wxr::PlaneOrientation::Unknown,
    }
}

/// A zeroed plane location with the header filled in, which is what the runtime is told to expect.
fn empty_location() -> openxr_sys::PlaneDetectorLocationEXT {
    openxr_sys::PlaneDetectorLocationEXT {
        ty: openxr_sys::StructureType::PLANE_DETECTOR_LOCATION_EXT,
        next: std::ptr::null_mut(),
        plane_id: 0,
        location_flags: openxr_sys::SpaceLocationFlags::EMPTY,
        pose: xr::Posef::IDENTITY,
        extents: openxr_sys::Extent2Df {
            width: 0.0,
            height: 0.0,
        },
        orientation: openxr_sys::PlaneDetectorOrientationEXT::ARBITRARY,
        semantic_type: openxr_sys::PlaneDetectorSemanticTypeEXT::UNDEFINED,
        polygon_buffer_count: 0,
    }
}

/// A null detector handle, which is what the out-parameter of the create call starts as.
fn null_detector() -> openxr_sys::PlaneDetectorEXT {
    // SAFETY: this is the null handle, and nothing dereferences it until the runtime has written one.
    unsafe { std::mem::zeroed() }
}

impl Drop for PlaneDetector {
    fn drop(&mut self) {
        // Anything this pass had left is dropped with it, which is what the spec says a destroy does - and
        // why the `plane_id` this backend reported is only good until this line, which is the same lifetime
        // the core gives a plane anyway.
        // SAFETY: the detector is this session's and is being destroyed exactly once.
        let result = unsafe { (self.table.destroy_plane_detector)(self.detector) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: destroying the plane detector: {result:?}");
        }
    }
}
