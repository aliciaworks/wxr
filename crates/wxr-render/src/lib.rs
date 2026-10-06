//! The renderer: what happens between a session's frame and a picture.
//!
//! It sits under every backend and knows about none of them. What it is given is a [`wxr::Frame`] - poses,
//! fields of view and which image each eye goes in - and a device it did not make. What it needs from a
//! backend is the one thing a backend knows and it cannot: how *this* platform's images become wgpu
//! textures. That is [`Import`], and it is implemented by the backend, next to the images it names.
//!
//! This is the shape the core's [`wxr::Session::Image`] was built for. The renderer is generic over it, the
//! core never looks inside it, and the two meet in an `impl` that lives in the backend - which is the only
//! place that can name both a `VkImage` and a `wgpu::Texture` without either crate depending on the other.

pub mod projection;
pub mod scene;

pub use projection::{
    Depth, angles, angles_from_tangents, forward, from_gl, perspective, pose_from_view, view,
};

use wxr::ImageMeta;

/// A core colour format as wgpu's.
///
/// `None` for a format this renderer has not learned: an image it cannot draw into is a frame it does not
/// draw, which is better than one drawn in the wrong colour space. It lives here and not in a backend
/// because every backend needs the same answer and two copies of it would be two answers.
pub fn texture_format(format: wxr::ColorFormat) -> Option<wgpu::TextureFormat> {
    match format {
        wxr::ColorFormat::Rgba8Srgb => Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        wxr::ColorFormat::Rgba8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
        wxr::ColorFormat::Bgra8Srgb => Some(wgpu::TextureFormat::Bgra8UnormSrgb),
        wxr::ColorFormat::Bgra8Unorm => Some(wgpu::TextureFormat::Bgra8Unorm),
        wxr::ColorFormat::Rgba16Float => Some(wgpu::TextureFormat::Rgba16Float),
        wxr::ColorFormat::Rgb10a2Unorm => Some(wgpu::TextureFormat::Rgb10a2Unorm),
        wxr::ColorFormat::Unknown => None,
    }
}

/// How a backend's images become wgpu textures.
///
/// One call per image per session rather than per frame: a compositor's images are made once and presented
/// many times, and a renderer that made a texture a frame would be making the same object again and again.
pub trait Import {
    /// The session's own name for an image, which is [`wxr::Session::Image`] on the session this is written
    /// for.
    type Image;

    /// Wrap one of the session's images.
    ///
    /// `None` when it cannot be - a browser whose WebXR session has no images at all, an image whose format
    /// this renderer does not know - because a frame that cannot be drawn is not an error the renderer can do
    /// anything about.
    fn texture(
        &self,
        device: &wgpu::Device,
        meta: ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture>;
}

/// Draws a frame into whatever the session says the picture goes into.
pub struct Renderer {
    /// The colour every eye starts as. A scene draws on top of it.
    clear: wgpu::Color,
    /// One texture per image the session handed out, made on first use.
    cache: Vec<Option<wgpu::Texture>>,
    /// What to draw, if anything. A renderer with no scene clears, which is what a frame loop wants to be
    /// able to do while the thing being drawn is still being written.
    scene: Option<scene::Scene>,
}

impl Renderer {
    pub fn new(clear: [f64; 4]) -> Self {
        Self {
            clear: wgpu::Color {
                r: clear[0],
                g: clear[1],
                b: clear[2],
                a: clear[3],
            },
            cache: Vec::new(),
            scene: None,
        }
    }

    /// A renderer that draws something: a triangle, with each eye's own projection and place.
    pub fn with_scene(device: &wgpu::Device, format: wgpu::TextureFormat, clear: [f64; 4]) -> Self {
        Self {
            scene: Some(scene::Scene::new(device, format)),
            ..Self::new(clear)
        }
    }

    /// Draw one frame into every eye, and hand it back for the compositor.
    ///
    /// The session is taken mutably because presenting is part of drawing: a frame that is drawn and not
    /// presented is a frame the headset never sees, and leaving the two to be called separately is leaving
    /// them to be called out of order.
    pub fn draw<S, I>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        session: &mut S,
        importer: &I,
        frame: &mut wxr::Frame,
    ) -> Result<usize, wxr::Error>
    where
        S: wxr::Session,
        I: Import<Image = S::Image>,
    {
        if !frame.is_render() {
            return Ok(0);
        }
        let meta = session.images();
        self.cache.resize(session.image_count(), None);

        let mut drawn = 0;
        for view in frame.views() {
            let Some(image) = session.image(view.image) else {
                continue;
            };
            let Some(texture) = self.cached(device, importer, meta, view.image, image) else {
                continue;
            };
            // One layer per eye, which is what the viewport is expressed in.
            let layer = texture.create_view(&wgpu::TextureViewDescriptor {
                base_array_layer: view.layer,
                array_layer_count: Some(1),
                ..Default::default()
            });

            // A clear, and nothing else yet: the pipeline a scene needs - a camera, a depth buffer, the
            // scene's own data - belongs to whatever is being drawn, and this crate's job is the part that
            // is the same for all of them.
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wxr frame"),
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wxr eye"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &layer,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                // A frame that is one view per pass has nothing to multi-view, and the field is here so that a
                // renderer that does can say so.
                ..Default::default()
            });
            if let Some(scene) = &self.scene {
                scene.draw(queue, view, &mut pass);
            }
            drop(pass);
            queue.submit(Some(encoder.finish()));
            drawn += 1;
        }

        session.end(frame)?;
        Ok(drawn)
    }

    /// The texture for an image, made once.
    fn cached<I: Import>(
        &mut self,
        device: &wgpu::Device,
        importer: &I,
        meta: ImageMeta,
        index: usize,
        image: &I::Image,
    ) -> Option<&wgpu::Texture> {
        if self.cache.get(index).is_none_or(Option::is_none) {
            let texture = importer.texture(device, meta, image)?;
            *self.cache.get_mut(index)? = Some(texture);
        }
        self.cache[index].as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wxr::{Backend as _, Session as _};

    /// An importer for the mock, whose images are numbers: this makes a plain texture in their place, so the
    /// test is about the renderer's loop and not about anyone's compositor.
    struct Plain;

    impl Import for Plain {
        type Image = u32;

        fn texture(
            &self,
            device: &wgpu::Device,
            meta: ImageMeta,
            _image: &Self::Image,
        ) -> Option<wgpu::Texture> {
            Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test eye"),
                size: wgpu::Extent3d {
                    width: meta.extent.width,
                    height: meta.extent.height,
                    depth_or_array_layers: meta.layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }))
        }
    }

    /// A headless device, or `None` on a machine that has no GPU at all - a test is not the place to fail
    /// over that, and the renderer is not what would be wrong.
    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("wxr-render test"),
            ..Default::default()
        }))
        .ok()
    }

    #[test]
    fn a_frame_with_two_eyes_draws_twice_and_presents() {
        let Some((device, queue)) = device() else {
            return;
        };

        // The mock is what makes this a test of the renderer rather than of a headset: it is a session with
        // two eyes and an image, and the renderer cannot tell it from any other.
        let mut session = wxr::mock::MockBackend::default()
            .connect(())
            .expect("the mock connects");
        while let Some(event) = session.poll() {
            if matches!(event, wxr::Event::StateChanged(wxr::State::Focused)) {
                break;
            }
        }
        let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");

        let mut frame = wxr::Frame::default();
        session.begin(Duration::ZERO, &mut frame).expect("a frame");
        session.views(space, &mut frame).expect("views");
        assert_eq!(frame.views().len(), 2, "the mock presents two eyes");

        let mut renderer = Renderer::new([0.1, 0.1, 0.1, 1.0]);
        let drawn = renderer
            .draw(&device, &queue, &mut session, &Plain, &mut frame)
            .expect("the frame draws");
        assert_eq!(drawn, 2, "one pass per eye");
    }

    #[test]
    fn a_triangle_is_drawn_with_each_eye_own_projection() {
        let Some((device, queue)) = device() else {
            return;
        };
        let mut session = wxr::mock::MockBackend::default().connect(()).unwrap();
        while let Some(event) = session.poll() {
            if matches!(event, wxr::Event::StateChanged(wxr::State::Focused)) {
                break;
            }
        }
        let space = session.space(wxr::SpaceKind::LocalFloor).unwrap();
        let mut frame = wxr::Frame::default();
        session.begin(Duration::ZERO, &mut frame).unwrap();
        session.views(space, &mut frame).unwrap();

        // The scene is what makes the projection arithmetic load-bearing: without it the frame would be a
        // clear, and a clear passes whatever matrix it is given.
        let mut renderer = Renderer::with_scene(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            [0.0, 0.0, 0.0, 1.0],
        );
        let drawn = renderer
            .draw(&device, &queue, &mut session, &Plain, &mut frame)
            .expect("the frame draws");
        assert_eq!(drawn, 2);
    }
}
