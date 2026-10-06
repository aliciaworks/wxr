//! Draw the scene into the compositor's images, over OpenXR.
//!
//! The headless example stops at the seam - a frame, two views and an image handle - and this one is
//! everything above it: the renderer wraps each image as a `wgpu` texture, draws the scene into it with that
//! eye's own projection and place, and hands the frame back. It is the same loop `wxr-apple`'s entry point
//! runs, on the platform whose compositor takes the picture as a Vulkan image the renderer makes a texture of.
//!
//! What it does *not* submit is depth: the binding this crate is on cannot chain a depth layer, so the
//! renderer makes its own depth buffer and the compositor gets colour alone - see the crate's module
//! documentation. Everything else about the frame is the real path, which is what makes this the example that
//! says whether the core's shape is workable rather than merely consistent.

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::time::{Duration, Instant};

    use wxr::{Backend as _, Session as _};

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        // A device with no surface: nothing is presented to a window, because the compositor is where the
        // picture goes. The queue is kept this time - drawing and presenting both need it.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("wxr-openxr rendered example"),
                ..Default::default()
            }))?;

        let backend = wxr_openxr::OpenXr::load()?;
        let mut session = backend.connect(
            wxr_openxr::Device {
                instance,
                device: device.clone(),
            },
            wxr::SessionMode::ImmersiveVr,
        )?;
        println!(
            "connected: {:?}, presentation {:?}, images {:?}",
            session.state(),
            session.presentation(),
            session.images()
        );

        // A floor to stand on when the runtime has one, and the head otherwise.
        let space = match session.space(wxr::SpaceKind::LocalFloor) {
            Ok(space) => space,
            Err(_) => session.space(wxr::SpaceKind::Viewer)?,
        };

        // The near and far planes the scene is built with. OpenXR does not submit depth through this binding
        // yet, so on this backend this only reaches the session - but it is the same call the other two get,
        // and the one the renderer's own depth buffer is built from.
        let (near, far) = wxr_render::scene::planes();
        session.set_depth_range(near, far);

        // Made before the first frame, which is the point of `images` being answerable without one.
        let meta = session.images();
        let Some(format) = wxr_render::texture_format(meta.format) else {
            return Err(format!(
                "the compositor's {:?} images are not a format this renderer knows",
                meta.format
            )
            .into());
        };
        let mut renderer = wxr_render::Renderer::with_scene(
            &device,
            format,
            // Not black: a frame that draws nothing should still be visibly a frame rather than a hole.
            [0.02, 0.02, 0.05, 1.0],
            // `0..w`, which is what Vulkan and wgpu use. The convention is the platform's, which is why it is
            // named here rather than assumed by the renderer.
            wxr_render::Depth::ZeroToOne,
        );

        let start = Instant::now();
        let mut frame = wxr::Frame::default();
        let mut inputs = Vec::new();
        let mut drawn = 0;
        // Bounded, because a null compositor never ends a session and an example is not a game.
        for _ in 0..600 {
            while let Some(event) = session.poll() {
                println!("event: {event:?}");
                if matches!(
                    event,
                    wxr::Event::StateChanged(wxr::State::Ended) | wxr::Event::Lost
                ) {
                    println!("the runtime ended the session after {drawn} frame(s)");
                    return Ok(());
                }
            }

            session.begin(start.elapsed(), &mut frame)?;
            if !frame.is_render() {
                // Waiting for the compositor to have a frame is a sleep rather than a spin; a real app waits
                // on the compositor's own blocking wait instead.
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }

            session.views(space, &mut frame)?;
            inputs.clear();
            session.inputs(space, &mut inputs)?;
            // Drawing presents: the frame is handed back at the end of it, which is why the two are one call.
            drawn += renderer.draw(
                &device,
                &queue,
                &mut session,
                &wxr_openxr::Images,
                &mut frame,
            )?;
        }

        println!("drew {drawn} frame(s)");
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
