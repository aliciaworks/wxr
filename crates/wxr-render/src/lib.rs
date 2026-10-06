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

pub use projection::{Depth, angles, forward, from_gl, perspective, view};

use wxr::ImageMeta;

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
    /// The colour to clear every eye to, which is all a first renderer draws. A scene goes here.
    clear: wgpu::Color,
    /// One texture per image the session handed out, made on first use.
    cache: Vec<Option<wgpu::Texture>>,
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
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
