//! The Rust half of an immersive app's entry point.
//!
//! An app on visionOS is a SwiftUI `ImmersiveSpace` whose `CompositorLayer` closure hands over a layer
//! renderer, and the closure is where Rust is entered. Everything above that belongs to the app: adopting the
//! compositor's device into wgpu (see [`AppleBackend::device`] for what is known about it) and deciding what
//! to draw. What is here is the loop in between, because it is the same loop for every app and getting its
//! order wrong is a session that never shows anything.
//!
//! The order is the compositor's: poll for a state change, begin a frame - which is where the timing and the
//! drawable come from - ask for the views and the inputs, draw, and give the frame back. [`run`] does exactly
//! that with this crate's own scene, so that the whole leg is runnable from one call. An app with a scene of
//! its own drives [`wxr::Session`] directly instead, and this function is the thirty lines it needs.

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2_compositor_services::cp_layer_renderer_t;

use wxr::{Backend as _, Session as _};

use crate::import::Images;
use crate::session::AppleBackend;

/// Run frames until the immersive space closes.
///
/// `update` is called once per frame that is being drawn, after the views and the inputs are known and before
/// the frame is drawn. That is the only place an app's own work belongs - it needs both of those to be worth
/// doing - and it is why this is a callback rather than something the loop guesses at.
pub fn run(
    layer_renderer: Retained<cp_layer_renderer_t>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    mut update: impl FnMut(&wxr::Frame, &[wxr::InputSource]),
) -> Result<(), wxr::Error> {
    let backend = AppleBackend::new(layer_renderer);
    // The queue goes to the session, because presenting is committing a command buffer on the renderer's own
    // queue; the device stays here, because drawing is this function's. It is cloned rather than moved
    // because drawing needs it too, and a `wgpu::Queue` is a handle.
    let mut session = backend.connect(queue.clone())?;
    // The planes the scene draws with, told to the compositor so that it can reproject with the depth the
    // renderer submits. One place and not two: the scene uses these numbers to build its projection, and the
    // compositor uses them to read what came out of it.
    let (near, far) = wxr_render::scene::planes();
    session.set_depth_range(near, far);

    // ARKit's origin when there is one, and the wearer's head otherwise. A space that was refused has to be
    // asked for as the one that exists rather than assumed.
    let space = match session.space(wxr::SpaceKind::Local) {
        Ok(space) => space,
        Err(_) => session.space(wxr::SpaceKind::Viewer)?,
    };

    // The renderer is made before the first frame, which the configuration makes possible: `images` can name
    // the format the layer was configured with, and the drawable only adds the size to it.
    let meta = session.images();
    let Some(format) = wxr_render::texture_format(meta.format) else {
        return Err(wxr::Error::Present(format!(
            "the compositor's {:?} images are not a format this renderer knows",
            meta.format
        )));
    };
    let mut renderer = wxr_render::Renderer::with_scene(
        &device,
        format,
        // Not black: a frame that draws nothing should still be visibly a frame rather than a hole.
        [0.02, 0.02, 0.05, 1.0],
        session.depth(),
    );

    let start = Instant::now();
    let mut frame = wxr::Frame::default();
    let mut inputs = Vec::new();
    loop {
        while let Some(event) = session.poll() {
            if matches!(
                event,
                wxr::Event::StateChanged(wxr::State::Ended) | wxr::Event::Lost
            ) {
                return Ok(());
            }
        }

        session.begin(start.elapsed(), &mut frame)?;
        if !frame.is_render() {
            // The layer is paused, or has no frame to give. Sleeping rather than spinning on it: the thing
            // worth waiting for is the compositor's own `wait_until_running`, which blocks the calling thread,
            // and a loop that wants the display's pacing calls it instead of this.
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        session.views(space, &mut frame)?;
        inputs.clear();
        session.inputs(space, &mut inputs)?;
        update(&frame, &inputs);
        renderer.draw(&device, &queue, &mut session, &Images, &mut frame)?;
    }
}
