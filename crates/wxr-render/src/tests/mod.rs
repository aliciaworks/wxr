//! What the renderer does with a session, exercised on the mock one.
use super::*;

/// The pixel at the centre of a pass that draws the near triangle first and the far one second.
///
/// The depth buffer is the only reason the near one is what comes back: without it the second triangle
/// would paint over the first, which is what makes the buffer load-bearing rather than decorative. Both
/// conventions are drawn, because a reverse projection compared with `Less` is a picture where nothing is
/// in front of anything - and that is the failure this pins.
fn centre_pixel_with(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    depth: Depth,
    occlusion: Option<(wgpu::Texture, scene::Occlusion)>,
) -> [u8; 3] {
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
        recommended_viewport_scale: None,
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
        scene.draw(device, queue, &eye, occlusion.as_ref(), &mut pass);
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

/// The centre pixel with nothing measured: the scene as it draws without a room.
fn centre_pixel(device: &wgpu::Device, queue: &wgpu::Queue, depth: Depth) -> [u8; 3] {
    centre_pixel_with(device, queue, depth, None)
}

/// A room of one distance, as a one-texel depth buffer.
///
/// A shader that reads a depth buffer needs nothing more than a value and the metadata that says what it is
/// worth - and `raw_value_to_meters` is 1, so the value *is* the distance in metres. A one-texel buffer is
/// therefore a room that is the same distance in every direction, which is all a test needs.
fn room(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    metres: f32,
) -> (wgpu::Texture, scene::Occlusion) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test room"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &metres.to_le_bytes(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let info = wxr::DepthInfo {
        size: wxr::Extent2d::new(1, 1),
        raw_value_to_meters: 1.0,
        norm_depth_buffer_from_norm_view: wxr::Pose::IDENTITY,
    };
    (texture, scene::Occlusion::from(info))
}

/// The room's depth is what makes a fragment behind it disappear, and this is the test that says the shader
/// does it rather than that it compiles.
///
/// The scene's triangles are 1 m and 2.5 m in front of the eye, so a room at half a metre is in front of both
/// and a room at five metres is behind them - and the picture has to differ from the one without a room in the
/// first case and be exactly it in the second. That "exactly" is the half worth having: an occlusion test that
/// only checked the first case would pass for a shader that discarded everything.
#[test]
fn a_fragment_behind_the_room_is_not_drawn() {
    let Some((device, queue)) = device() else {
        return;
    };
    let drawn = centre_pixel(&device, &queue, Depth::ZeroToOne);

    let blocked = centre_pixel_with(
        &device,
        &queue,
        Depth::ZeroToOne,
        Some(room(&device, &queue, 0.5)),
    );
    assert_ne!(
        blocked, drawn,
        "a room in front of everything hides the scene"
    );

    let behind = centre_pixel_with(
        &device,
        &queue,
        Depth::ZeroToOne,
        Some(room(&device, &queue, 5.0)),
    );
    assert_eq!(behind, drawn, "a room behind everything changes nothing");
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
    type Depth = ();

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
        .connect((), wxr::SessionMode::ImmersiveVr)
        .expect("the mock connects");
    while let Some(event) = session.poll() {
        if matches!(
            event,
            wxr::Event::VisibilityChanged(wxr::Visibility::Visible)
        ) {
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
    type Depth = ();

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
        .connect((), wxr::SessionMode::ImmersiveVr)
        .expect("the mock connects");
    while let Some(event) = session.poll() {
        if matches!(
            event,
            wxr::Event::VisibilityChanged(wxr::Visibility::Visible)
        ) {
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
    type Depth = ();

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

mod frames;
