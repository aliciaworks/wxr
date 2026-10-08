//! Make a composition layer on the runtime this machine has, and hand it back.
//!
//! The frame loop cannot run on a machine with no display - `xrWaitFrame` waits for a session that never
//! becomes visible - so `headless` proves the seam and stops at the frame it cannot get. A *layer* needs no
//! frame: `XR_KHR_composition_layer_*` layers aside, a quad is in the core specification, and its swapchain is
//! created against a session that is merely idle. So this is what can be verified without a headset: that the
//! runtime makes a swapchain of the shape a quad names, that its images come back in the size and format the
//! app asked for, and that letting it go is enough to be rid of it.
//!
//! What it does *not* prove is a layer on a display: that is `xrEndFrame` with a quad in the list, and it needs
//! a session that is visible. It is written down here so the gap is a known one.

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::time::Duration;

    use wxr::{Backend, Session as _, SpaceKind};

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("wxr-openxr layers example"),
                ..Default::default()
            }))?;

        let backend = wxr_openxr::OpenXr::load()?;
        let mut session = backend.connect(
            wxr_openxr::Device {
                instance: instance.clone(),
                device,
            },
            wxr::SessionMode::ImmersiveVr,
        )?;

        // The events have to be drained for the session to climb its ladder, and OpenXR's ladder is where
        // `IDLE` becomes `READY` - which is the state a swapchain may be made in.
        while let Some(event) = session.poll() {
            println!("event: {event:?}");
        }

        println!("features: {:?}", session.features());
        if !session.features().contains(wxr::Features::LAYER_QUAD) {
            println!("this runtime has no quad layer, so there is nothing to check");
            return Ok(());
        }

        let space = session.space(SpaceKind::LocalFloor)?;
        let layer = session.layer(
            space,
            wxr::LayerShape::Quad {
                width: 1.2,
                height: 0.8,
            },
            wxr::Extent2d::new(1024, 683),
        )?;
        println!("made a quad layer: {layer:?}");

        // Placed, and placed again: on OpenXR that is a value the next `xrEndFrame` reads rather than a call
        // into the runtime, which is why this proving nothing yet is the honest outcome.
        session.set_layer_pose(
            layer,
            wxr::Pose {
                position: wxr::glam::Vec3::new(0.0, 1.5, -2.0),
                orientation: wxr::glam::Quat::IDENTITY,
            },
        )?;
        println!("placed it");

        session.release_layer(layer);
        println!("released it");

        // The other three shapes, whichever of them this runtime can take - the quad was made above. The three beyond the quad are
        // one `XR_KHR_composition_layer_*` extension each, enabled when the instance is made or not at all -
        // so the answer is the instance's, and a shape that is not there is refused rather than quietly made
        // into a quad. Which is what this prints: the capability bits say the same thing before the asking.
        let shapes = [
            (
                wxr::LayerShape::Cylinder {
                    radius: 1.0,
                    central_angle: 1.0,
                    aspect: 0.5,
                },
                wxr::Extent2d::new(1024, 512),
            ),
            (
                wxr::LayerShape::Equirect {
                    radius: 1.0,
                    central_horizontal: 1.0,
                    upper_vertical: 0.5,
                    lower_vertical: 0.5,
                },
                wxr::Extent2d::new(2048, 1024),
            ),
            (wxr::LayerShape::Cube, wxr::Extent2d::new(512, 512)),
        ];
        for (shape, pixels) in shapes {
            match session.layer(space, shape, pixels) {
                Ok(made) => {
                    println!("made a {} layer: {made:?}", shape.name());
                    session.release_layer(made);
                }
                Err(error) => println!("a {} layer: {error}", shape.name()),
            }
        }

        // A frame would be the proof of the rest, and there is none to be had here.
        let _ = Duration::ZERO;
        Ok(())
    }
}

fn main() {
    #[cfg(not(target_family = "wasm"))]
    native::run().expect("the example failed");
    #[cfg(target_family = "wasm")]
    println!("wxr-openxr: native only; the WebXR backend is `wxr-webxr`");
}
