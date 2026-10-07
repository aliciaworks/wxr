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
    Depth, angles, angles_from_tangents, forward, from_gl, perspective, pose_from_transform,
    pose_from_view, view,
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

/// The depth format every pass in this renderer uses.
///
/// One format and not a choice: `Depth32Float` is what every compositor this workspace draws into can take -
/// visionOS's asks for it by name - and a depth buffer swapped per platform is one no pass can be written
/// against.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// What a depth buffer does for a projection convention.
///
/// The two fields are one decision and have to agree: a reverse projection puts the near plane at one and the
/// far at zero, so nearer is *greater* and an empty buffer is zero. Getting one right and the other wrong is a
/// picture where everything hides behind everything, or nothing does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DepthState {
    pub compare: wgpu::CompareFunction,
    pub clear: f32,
}

/// The depth state a projection convention needs.
pub fn depth_state(depth: Depth) -> DepthState {
    match depth {
        Depth::Reverse => DepthState {
            compare: wgpu::CompareFunction::Greater,
            clear: 0.0,
        },
        Depth::ZeroToOne | Depth::MinusOneToOne => DepthState {
            compare: wgpu::CompareFunction::Less,
            clear: 1.0,
        },
    }
}

/// How a backend's images become wgpu textures.
///
/// One call per image per *frame*, because a compositor's handle may be: a `CompositorServices` drawable's
/// textures and a WebXR sub-image's belong to the frame they came with, and OpenXR's is whichever image the
/// frame acquired. A renderer that kept the first one would draw into a picture the compositor has already
/// shown, so there is no cache and no assumption - what the session hands over now is what is wrapped now.
///
/// An importer is a **value of its own** rather than the session it belongs to, and that is a constraint
/// [`Renderer::draw`] puts on it rather than a style: `draw` needs the session mutably - drawing is
/// presenting - and the importer at the same time, so a backend that implemented this on its session would
/// have an importer it could never pass. A backend writes it on a marker type, which is also what makes it
/// obvious that there is nothing in it: wrapping an image is a function of the image and the device.
pub trait Import {
    /// The session's own name for an image, which is [`wxr::Session::Image`] on the session this is written
    /// for.
    type Image;

    /// The session's own name for a depth buffer, when it has one - [`wxr::Session::Depth`] on the session this
    /// is written for. `()` for a backend with none, which is what a session that never measured anything is.
    type Depth;

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

    /// The depth buffer this frame should be drawn with, if the session has one to offer.
    ///
    /// A compositor that gets depth back can reproject the frame - move it to match where the head really is
    /// when the picture reaches the display - and two of the three here hand one over: a `CompositorServices`
    /// drawable has a depth texture, and a WebXR/WebGPU sub-image has a `depthStencilTexture`. Drawing into
    /// the compositor's own is the whole difference between submitting depth and keeping it.
    ///
    /// `None` - the default - is a session with no depth to give, and then the renderer makes its own. So is a
    /// session whose depth is not the format this renderer's pipeline was built for: the renderer decides
    /// that per view, and falling back is deliberate.
    fn depth(
        &self,
        _device: &wgpu::Device,
        _meta: ImageMeta,
        _image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        None
    }

    /// The room's depth for a view, as a texture to test fragments against, when the session measured one.
    ///
    /// [`Import::depth`] above is the buffer a pass is *drawn into*; this is the measurement of the real world
    /// beside it, and it is what a scene throws occluded fragments away against. The metadata comes along
    /// because an importer needs the buffer's size to describe it, and a shader needs what its values mean.
    ///
    /// `None` - the default - is a session with no depth to test against, and then the scene draws everything.
    fn session_depth(
        &self,
        _device: &wgpu::Device,
        _info: wxr::DepthInfo,
        _depth: &Self::Depth,
    ) -> Option<wgpu::Texture> {
        None
    }
}

/// Draws a frame into whatever the session says the picture goes into.
pub struct Renderer {
    /// The colour every eye starts as. A scene draws on top of it.
    clear: wgpu::Color,
    /// What to draw, if anything. A renderer with no scene clears, which is what a frame loop wants to be
    /// able to do while the thing being drawn is still being written.
    scene: Option<scene::Scene>,
    /// One private depth buffer per image, made on first use - the same shape as the image it tests against,
    /// and what a session with no depth of its own gets.
    depth_textures: Vec<Option<wgpu::Texture>>,
    /// The session's depth for the view being drawn, kept only as long as the view is: a compositor's depth
    /// belongs to the frame it came with, like its colour does.
    session_depth: Option<wgpu::Texture>,
    /// The convention the target insists on, which decides how the depth buffer compares and what an empty
    /// one is worth.
    depth: Depth,
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
            scene: None,
            depth_textures: Vec::new(),
            session_depth: None,
            depth: Depth::default(),
        }
    }

    /// A renderer that draws something: a triangle, with each eye's own projection and place.
    ///
    /// `depth` is the convention the target insists on - [`Depth::ZeroToOne`] everywhere but visionOS, whose
    /// compositor drawables are reverse-Z. It is an argument rather than a default because drawing into one
    /// with the wrong convention is a wrong picture rather than a compilation error.
    pub fn with_scene(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        clear: [f64; 4],
        depth: Depth,
    ) -> Self {
        Self {
            scene: Some(scene::Scene::new(device, format, depth)),
            depth,
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
        I: Import<Image = S::Image, Depth = S::Depth>,
    {
        if !frame.is_render() {
            return Ok(0);
        }
        let meta = session.images();
        self.depth_textures.resize(session.image_count(), None);
        let clear_depth = depth_state(self.depth).clear;

        let mut drawn = 0;
        for (index, view) in frame.views().iter().enumerate() {
            // The room's depth, if this session measured any: imported and let go of in one breath, because the
            // buffer belongs to the frame and the session cannot be asked for anything else while a reference
            // into it is held.
            let occlusion = session.depth(index).and_then(|(depth, info)| {
                importer
                    .session_depth(device, info, depth)
                    .map(|texture| (texture, scene::Occlusion::from(info)))
            });
            let Some(image) = session.image(view.image) else {
                continue;
            };
            // Wrapped here, per view, per frame: the session's handle for *this* frame is the only one that
            // can be drawn into, and a cache would make the first frame's textures outlive their frame.
            let Some(texture) = importer.texture(device, meta, image) else {
                continue;
            };
            // One layer per eye, which is what the viewport is expressed in.
            let layer = texture.create_view(&wgpu::TextureViewDescriptor {
                base_array_layer: view.layer,
                array_layer_count: Some(1),
                ..Default::default()
            });
            let Some(depth) = self.depth_for(device, importer, meta, view.image, image) else {
                continue;
            };
            let depth_layer = depth.create_view(&wgpu::TextureViewDescriptor {
                base_array_layer: view.layer,
                array_layer_count: Some(1),
                ..Default::default()
            });

            // The colour, the depth, and nothing else: what a scene *is* belongs to whatever is being drawn,
            // and this crate's job is the part that is the same for all of them - the attachments, and what
            // an empty buffer is worth in the convention this target insists on.
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_layer,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_depth),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                // A frame that is one view per pass has nothing to multi-view, and the field is here so that a
                // renderer that does can say so.
                ..Default::default()
            });
            if let Some(scene) = &self.scene {
                scene.draw(device, queue, view, occlusion.as_ref(), &mut pass);
            }
            drop(pass);
            queue.submit(Some(encoder.finish()));
            drawn += 1;
        }

        session.end(frame)?;
        Ok(drawn)
    }

    /// Draw a layer's picture, once, into the image the compositor handed over.
    ///
    /// A layer is not an eye and not a view: the compositor places it and warps it for the optics, which is the
    /// whole reason to hand it one - so it is drawn once, and there is no [`wxr::View`] to draw it with. What
    /// decides how it is seen is the content, which is why the drawing is a callback and not a scene, and what
    /// is here is the part every layer shares: the image, the attachments, and what an empty one is worth.
    ///
    /// `false` is a session that handed over an image this renderer cannot wrap, which is a frame with no
    /// layer rather than a frame to fail - the same answer the eyes get.
    pub fn draw_layer<I: Import>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        importer: &I,
        meta: wxr::ImageMeta,
        image: &I::Image,
        layer: wxr::LayerImage,
        draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) -> bool {
        let Some(texture) = importer.texture(device, meta, image) else {
            return false;
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wxr layer"),
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wxr layer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Cleared and not loaded: the compositor's image is the compositor's, and nothing here knows
                    // what is in it. A panel's empty parts are nothing, which is what `Renderer::new`'s colour
                    // says - and the alpha in it is why a layer has to be presented with source alpha.
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            // No depth: a layer is one picture with nothing behind it, and the compositor has the world for
            // that.
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            ..Default::default()
        });
        // The part of the image that is this layer's, which is all of it on a platform that gives a layer its
        // own texture and a sub-rectangle on one that pools them.
        pass.set_viewport(
            layer.viewport.x as f32,
            layer.viewport.y as f32,
            layer.viewport.width as f32,
            layer.viewport.height as f32,
            0.0,
            1.0,
        );
        draw(&mut pass);
        drop(pass);
        queue.submit(Some(encoder.finish()));
        true
    }

    /// The depth buffer for this view: the session's if it offered one this renderer can draw into, and a
    /// private one otherwise.
    ///
    /// The format is what decides it, and the check is not a formality: a pipeline declares the depth format it
    /// was built for, and a pass has to attach one of that format - so a compositor handing over another one is
    /// a validation error rather than a frame. Falling back to a private buffer keeps the frame and loses only
    /// the reprojection, which is the right way round.
    fn depth_for<I: Import>(
        &mut self,
        device: &wgpu::Device,
        importer: &I,
        meta: ImageMeta,
        index: usize,
        image: &I::Image,
    ) -> Option<&wgpu::Texture> {
        if let Some(session) = importer.depth(device, meta, image) {
            if session.format() == DEPTH_FORMAT {
                self.session_depth = Some(session);
                return self.session_depth.as_ref();
            }
            log::warn!(
                "wxr-render: this session's depth is {:?} and the pipeline was built for {DEPTH_FORMAT:?};                  drawing into a private one, which loses the reprojection and not the frame",
                session.format()
            );
        }
        self.private_depth(device, meta, index)
    }

    /// The renderer's own depth buffer for an image, made once: the same extent and the same layers as the
    /// colour image, so that a view's array layer means the same thing in both.
    fn private_depth(
        &mut self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        index: usize,
    ) -> Option<&wgpu::Texture> {
        if self.depth_textures.get(index).is_none_or(Option::is_none) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("wxr depth"),
                size: wgpu::Extent3d {
                    width: meta.extent.width,
                    height: meta.extent.height,
                    depth_or_array_layers: meta.layers.max(1),
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            *self.depth_textures.get_mut(index)? = Some(texture);
        }
        self.depth_textures[index].as_ref()
    }
}

#[cfg(test)]
mod tests;
