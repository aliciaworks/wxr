//! Ask the runtime what the room is, and say which of the four ways it can answer.
//!
//! This is the example the other three are not: `headless` proves a frame arrives, `layers` proves a shape
//! can be handed over, and `rendered` proves a picture reaches an image. None of them touches the room. So
//! this one drives all four of the world-modelling extensions - the surfaces it has found, the place an
//! anchor keeps, where a ray meets something, and how far away the walls are - and prints what came back
//! from each.
//!
//! It needs `XR_EXT_plane_detection`, `XR_FB_spatial_entity`, `XR_META_environment_raycast` with
//! `XR_EXT_future`, and `XR_META_environment_depth`. A runtime with none of them prints four `no`s and four
//! refusals and exits cleanly, which is the honest answer and also what Monado does - so this runs on a
//! machine that has no headset and says exactly that. On a machine that has them, it is the one command that
//! exercises the four state machines against hardware rather than against a type.
//!
//! The frame loop is why it can: depth is only handed over between a frame's begin and its end, planes are a
//! state machine that advances a step per call, and both the anchor's completion event and the raycaster's
//! future only move when the session is polled. A test with a clock but no compositor would reach none of it.

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::time::Duration;

    use wxr::{Backend as _, Session as _, SpaceKind};

    /// Frames to run before giving up on anything that has not answered.
    ///
    /// A bound and not an expectation, for the same reason `tests/spatial.rs` has one: a runtime that never
    /// answers is the common case here, and this should take a second and end rather than hang.
    const FRAMES: usize = 120;

    /// What the run saw, so that the end of it is one answer rather than a scroll of lines.
    #[derive(Default)]
    struct Seen {
        planes: usize,
        anchor: Option<wxr::Pose>,
        hits: usize,
        depth: usize,
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        // The same headless device as `headless`: nothing is presented to a window, because the compositor
        // is where the picture goes.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("wxr-openxr room example"),
                ..Default::default()
            }))?;

        let backend = wxr_openxr::OpenXr::load()?;
        let mut session = backend.connect(
            wxr_openxr::Device {
                instance: instance.clone(),
                device,
            },
            wxr::SessionMode::ImmersiveAr,
        )?;

        // The four bits come first and on their own, because on a runtime that has none of them everything
        // below is four refusals - and a reader should be able to see that the refusals are the answer
        // rather than the failure.
        let features = session.features();
        println!("the session offers:");
        for (bit, name) in [
            (wxr::Features::PLANES, "planes"),
            (wxr::Features::ANCHORS, "anchors"),
            (wxr::Features::HIT_TEST, "hit test"),
            (wxr::Features::DEPTH, "depth"),
        ] {
            println!(
                "  {name:>9}: {}",
                if features.contains(bit) { "yes" } else { "no" }
            );
        }

        let space = session.space(SpaceKind::LocalFloor)?;
        let mut frame = wxr::Frame::default();
        let mut seen = Seen::default();

        // The two that are made once and then asked about every frame. A refusal either way is the answer to
        // "can this runtime do it", which the bits above have already given - so it is reported and the run
        // carries on rather than stopping.
        let source = match session.hit_test_source(space) {
            Ok(source) => Some(source),
            Err(error) => {
                println!("hit-test source: {error}");
                None
            }
        };
        let anchor = match session.anchor(space, wxr::Pose::IDENTITY) {
            Ok(anchor) => Some(anchor),
            Err(error) => {
                println!("anchor: {error}");
                None
            }
        };
        // The anchor arrives as an event and the raycaster as a future, and both are only moved by a poll -
        // so asking here and reading in the loop is the shape, not a missing call.
        if anchor.is_some() {
            println!("anchor requested; it exists once the runtime says so");
        }

        let mut planes = Vec::new();
        let mut hits = Vec::new();
        let mut reported_planes = usize::MAX;

        for _ in 0..FRAMES {
            while let Some(event) = session.poll() {
                println!("event: {event:?}");
            }
            session.begin(Duration::ZERO, &mut frame)?;
            if !frame.is_render() {
                continue;
            }
            // Before anything else, because the frame is what the four below are asked *in* - the projection
            // layer is built from these views at `end`, and a frame submitted with none is a validation
            // failure at the runtime rather than an empty picture.
            session.views(space, &mut frame)?;

            // One step of the plane state machine per frame: it begins a pass, and the planes come out of
            // the *next* call that finds the pass done. A rising count is a pass that finished.
            session.planes(space, &mut planes)?;
            if planes.len() != reported_planes {
                reported_planes = planes.len();
                println!("planes: {} found", planes.len());
                for plane in &planes {
                    println!(
                        "  {} {:?} at {:?}, {:.2} by {:.2} m",
                        plane.id,
                        plane.orientation,
                        plane.pose.position,
                        plane.extent.x,
                        plane.extent.y
                    );
                }
                seen.planes = reported_planes;
            }

            if let Some(source) = source {
                session.hits(source, space, &mut hits)?;
                for hit in &hits {
                    println!("ray met {:?}", hit.pose.position);
                    seen.hits += 1;
                }
            }

            if let Some(anchor) = anchor
                && let Some(pose) = session.anchor_pose(anchor, space)?
                && seen.anchor.is_none()
            {
                println!("the anchor is at {:?}", pose.position);
                seen.anchor = Some(pose);
            }

            // The first ask of a frame takes the image and both views read it; a second would be
            // `XR_ERROR_LIMIT_REACHED`, which is why this is here and not in a loop over the eyes.
            if let Some((image, info)) = session.depth(0) {
                if seen.depth == 0 {
                    println!(
                        "depth: {}x{} px, {:?}, image {image:?}",
                        info.size.width, info.size.height, info.encoding
                    );
                }
                seen.depth += 1;
            }

            session.end(&mut frame)?;
        }

        // Both are given back. The anchor's runtime object is destroyed by dropping it; the hit-test
        // source has no runtime object at all on this backend - the raycaster is session-wide and takes
        // the ray with each call - so `release_hit_test_source` is the slot the ray was measured from
        // being cleared, and the two lines below are what WebXR does with `XRHitTestSource.cancel`.
        if let Some(anchor) = anchor {
            session.release_anchor(anchor);
        }
        if let Some(source) = source {
            session.release_hit_test_source(source);
        }

        println!();
        println!("after {FRAMES} frames at most:");
        println!("  planes:  {}", seen.planes);
        println!(
            "  anchor:  {}",
            match seen.anchor {
                Some(pose) => format!("at {:?}", pose.position),
                None => "not locatable (never made, lost, or released)".to_string(),
            }
        );
        println!("  rays:    {} hit", seen.hits);
        println!("  depth:   {} frames measured", seen.depth);
        println!("  sensing: {:?}", session.depth_sensing());
        Ok(())
    }
}

fn main() {
    #[cfg(not(target_family = "wasm"))]
    native::run().expect("the example failed");
    // The OpenXR backend is native-only, so on a browser there is nothing here to run.
    #[cfg(target_family = "wasm")]
    println!("wxr-openxr: native only; the WebXR backend is `wxr-webxr`");
}
