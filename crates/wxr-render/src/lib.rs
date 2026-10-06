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
        I: Import<Image = S::Image>,
    {
        if !frame.is_render() {
            return Ok(0);
        }
        let meta = session.images();
        self.depth_textures.resize(session.image_count(), None);
        let clear_depth = depth_state(self.depth).clear;

        let mut drawn = 0;
        for view in frame.views() {
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
                scene.draw(queue, view, &mut pass);
            }
            drop(pass);
            queue.submit(Some(encoder.finish()));
            drawn += 1;
        }

        session.end(frame)?;
        Ok(drawn)
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
mod tests {
    use super::*;

    /// The pixel at the centre of a pass that draws the near triangle first and the far one second.
    ///
    /// The depth buffer is the only reason the near one is what comes back: without it the second triangle
    /// would paint over the first, which is what makes the buffer load-bearing rather than decorative. Both
    /// conventions are drawn, because a reverse projection compared with `Less` is a picture where nothing is
    /// in front of anything - and that is the failure this pins.
    fn centre_pixel(device: &wgpu::Device, queue: &wgpu::Queue, depth: Depth) -> [u8; 3] {
        let (width, height) = (64u32, 64u32);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let colour = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test eye"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let scene = scene::Scene::new(device, format, depth);
        let eye = wxr::View {
            eye: wxr::Eye::Mono,
            pose: wxr::Pose::IDENTITY,
            fov: scene::DEFAULT_FOV,
            viewport: wxr::Viewport {
                x: 0,
                y: 0,
                width,
                height,
            },
            image: 0,
            layer: 0,
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let colour_view = colour.create_view(&Default::default());
            let depth_view = depth_texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test eye"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &colour_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth_state(depth).clear),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            scene.draw(queue, &eye, &mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &colour,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: width / 2,
                    y: height / 2,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        let data = slice.get_mapped_range().expect("the readback is mapped");
        let pixel = [data[0], data[1], data[2]];
        drop(data);
        readback.unmap();
        pixel
    }

    /// An importer that remembers the images it was asked to wrap.
    ///
    /// Which is how a test can tell a renderer that wraps *this* frame's texture from one that kept the first
    /// frame's - and the difference is not academic: a compositor recycles its handles, so the first frame's
    /// texture is, by the third frame, a picture that has already been shown.
    #[derive(Default)]
    struct Recording {
        seen: std::cell::RefCell<Vec<u32>>,
    }

    impl Import for Recording {
        type Image = u32;

        fn texture(
            &self,
            device: &wgpu::Device,
            meta: ImageMeta,
            image: &Self::Image,
        ) -> Option<wgpu::Texture> {
            self.seen.borrow_mut().push(*image);
            Plain.texture(device, meta, image)
        }
    }

    #[test]
    fn every_frame_is_wrapped_from_its_own_image() {
        let Some((device, queue)) = device() else {
            return;
        };
        let mut session = wxr::mock::MockBackend::default()
            .connect(())
            .expect("the mock connects");
        while let Some(event) = session.poll() {
            if matches!(event, wxr::Event::StateChanged(wxr::State::Focused)) {
                break;
            }
        }
        let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");

        let recording = Recording::default();
        let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
        let mut frame = wxr::Frame::default();

        let mut expected = Vec::new();
        for _ in 0..3 {
            session.begin(Duration::ZERO, &mut frame).unwrap();
            session.views(space, &mut frame).unwrap();
            // What this frame handed over, which is what the renderer has to wrap.
            for view in frame.views() {
                expected.push(*session.image(view.image).expect("this frame's image"));
            }
            renderer
                .draw(&device, &queue, &mut session, &recording, &mut frame)
                .expect("the frame draws");
        }

        let seen = recording.seen.borrow();
        assert_eq!(
            *seen, expected,
            "every frame's own image rather than the first frame's"
        );
        assert!(
            seen.windows(2).any(|pair| pair[0] != pair[1]),
            "and the mock hands out a different image each frame, or this would say nothing: {seen:?}"
        );
    }

    #[test]
    fn the_nearer_triangle_is_the_one_that_shows() {
        let Some((device, queue)) = device() else {
            return;
        };
        let expected = [
            (scene::NEAR[0] * 255.0).round() as u8,
            (scene::NEAR[1] * 255.0).round() as u8,
            (scene::NEAR[2] * 255.0).round() as u8,
        ];
        for depth in [Depth::ZeroToOne, Depth::Reverse] {
            let pixel = centre_pixel(&device, &queue, depth);
            for (got, want) in pixel.iter().zip(expected.iter()) {
                assert!(
                    got.abs_diff(*want) <= 2,
                    "{depth:?}: read {pixel:?}, and the near triangle is {expected:?}"
                );
            }
        }
    }

    /// An importer that offers a depth buffer of a format this renderer's pipeline was not built for.
    ///
    /// A pass whose depth attachment disagrees with the format the pipeline declares is a validation error,
    /// and wgpu's uncaptured-error handler panics on one - so a frame that draws *anyway* is a frame that
    /// noticed the disagreement and used its own buffer. The scene is what makes the test say anything: with
    /// nothing drawing, there is no pipeline for the attachment to disagree with.
    struct WrongDepth;

    impl Import for WrongDepth {
        type Image = u32;

        fn texture(
            &self,
            device: &wgpu::Device,
            meta: ImageMeta,
            image: &Self::Image,
        ) -> Option<wgpu::Texture> {
            Plain.texture(device, meta, image)
        }

        fn depth(
            &self,
            device: &wgpu::Device,
            meta: ImageMeta,
            _image: &Self::Image,
        ) -> Option<wgpu::Texture> {
            Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("the wrong depth"),
                size: wgpu::Extent3d {
                    width: meta.extent.width.max(1),
                    height: meta.extent.height.max(1),
                    depth_or_array_layers: meta.layers.max(1),
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }))
        }
    }

    #[test]
    fn a_session_depth_of_another_format_falls_back_to_a_private_one() {
        let Some((device, queue)) = device() else {
            return;
        };
        let mut session = wxr::mock::MockBackend::default()
            .connect(())
            .expect("the mock connects");
        while let Some(event) = session.poll() {
            if matches!(event, wxr::Event::StateChanged(wxr::State::Focused)) {
                break;
            }
        }
        let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");

        // The mock's colour format, so that the only thing the pass and the pipeline can disagree about is
        // the depth buffer - which is the disagreement this test is about.
        let mut renderer = Renderer::with_scene(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            [0.0, 0.0, 0.0, 1.0],
            Depth::ZeroToOne,
        );
        let mut frame = wxr::Frame::default();
        session.begin(Duration::ZERO, &mut frame).unwrap();
        session.views(space, &mut frame).unwrap();

        let drawn = renderer
            .draw(&device, &queue, &mut session, &WrongDepth, &mut frame)
            .expect("the frame draws");
        assert_eq!(drawn, 2, "both eyes, on the renderer's own depth");
    }

    #[test]
    fn a_reverse_projection_compares_the_other_way_round() {
        // The two halves of one decision: nearer is smaller in a `0..1` projection and larger in a reverse
        // one, and an empty buffer is the far end of whichever it is.
        assert_eq!(
            depth_state(Depth::ZeroToOne).compare,
            wgpu::CompareFunction::Less
        );
        assert_eq!(depth_state(Depth::ZeroToOne).clear, 1.0);
        assert_eq!(
            depth_state(Depth::Reverse).compare,
            wgpu::CompareFunction::Greater
        );
        assert_eq!(depth_state(Depth::Reverse).clear, 0.0);
    }

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
            Depth::ZeroToOne,
        );
        let drawn = renderer
            .draw(&device, &queue, &mut session, &Plain, &mut frame)
            .expect("the frame draws");
        assert_eq!(drawn, 2);
    }
}
