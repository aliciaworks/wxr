//! The frame loop, as the renderer draws it: one pass per eye, presented through the session.
//!
//! It is the end-to-end test of this crate - a mock session with two eyes and an image, a renderer
//! with a scene, and a frame that comes out of it - and it is its own file because it is the one that
//! needs everything the others set up in pieces.

use super::*;

#[test]
fn a_frame_with_two_eyes_draws_twice_and_presents() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };

    // The mock is what makes this a test of the renderer rather than of a headset: it is a session with
    // two eyes and an image, and the renderer cannot tell it from any other.
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
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = wxr::mock::MockBackend::default()
        .connect((), wxr::SessionMode::ImmersiveVr)
        .unwrap();
    while let Some(event) = session.poll() {
        if matches!(
            event,
            wxr::Event::VisibilityChanged(wxr::Visibility::Visible)
        ) {
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

/// A session at the rung where it can draw, with images of a given shape.
fn session_at(extent: wxr::Extent2d, stereo: bool) -> wxr::mock::MockSession {
    let mut session = wxr::mock::MockBackend {
        extent,
        stereo,
        ..Default::default()
    }
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
    session
}

#[test]
fn a_depth_buffer_follows_the_image_it_is_attached_to() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut renderer = Renderer::with_scene(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        [0.0, 0.0, 0.0, 1.0],
        Depth::ZeroToOne,
    );
    let mut frame = wxr::Frame::default();

    // The cache is per image and an image keeps its index while its shape changes. A browser resizing its
    // canvas does exactly this: one image, a new extent. One layer then two is the other half of the same
    // problem - the second eye asks for a depth layer the first buffer never had, and the pass refuses a
    // view of it before the size is even considered. A renderer that kept the first buffer would fail both.
    for (extent, stereo) in [
        (wxr::Extent2d::new(64, 64), false),
        (wxr::Extent2d::new(64, 64), true),
        (wxr::Extent2d::new(32, 48), true),
    ] {
        let mut session = session_at(extent, stereo);
        let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
        session.begin(Duration::ZERO, &mut frame).expect("a frame");
        session.views(space, &mut frame).expect("views");
        renderer
            .draw(&device, &queue, &mut session, &Plain, &mut frame)
            .unwrap_or_else(|error| panic!("{extent:?}, stereo {stereo}: {error}"));
    }
}

#[test]
fn an_eye_with_an_empty_viewport_is_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(64, 64), true);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");
    frame.views_mut()[0].viewport.width = 0;

    let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame still ends");
    assert_eq!(drawn, 1, "the other eye is still a view with pixels");
}

/// A viewport that reaches past the image is a pass wgpu refuses, not an eye drawn on its right-hand part.
#[test]
fn an_eye_viewport_outside_the_image_is_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(64, 64), true);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");
    frame.views_mut()[0].viewport.x = 32;

    let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame still ends");
    assert_eq!(drawn, 1, "the other eye is still inside the image");
}

/// A frame whose image has no pixels is a frame with nothing in it, not a texture to create.
#[test]
fn a_frame_with_an_empty_image_is_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(0, 0), false);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");

    let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame still ends");
    assert_eq!(drawn, 0, "there is no image to draw into");
}

/// An eye naming a layer the image does not have is malformed, and drawing it would be a validation error.
#[test]
fn an_eye_past_the_images_layers_is_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(64, 64), true);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");
    frame.views_mut()[0].layer = 2;

    let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
    let drawn = renderer
        .draw(&device, &queue, &mut session, &Plain, &mut frame)
        .expect("the frame still ends");
    assert_eq!(drawn, 1, "the other eye is still a layer of this image");
}

/// An importer that offers the compositor's depth as well as its colour.
///
/// A drawable's depth belongs to the frame it came with, exactly as its colour does, so the interesting
/// question is whether the renderer still holds it once the frame has been handed back.
struct CompositorDepth;

impl Import for CompositorDepth {
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
            label: Some("compositor depth"),
            size: wgpu::Extent3d {
                width: meta.extent.width.max(1),
                height: meta.extent.height.max(1),
                depth_or_array_layers: meta.layers.max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }))
    }
}

#[test]
fn the_compositor_depth_is_let_go_when_the_frame_is_given_back() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(64, 64), true);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");

    let mut renderer = Renderer::new([0.0, 0.0, 0.0, 1.0]);
    renderer
        .draw(&device, &queue, &mut session, &CompositorDepth, &mut frame)
        .expect("the frame draws");

    assert!(
        renderer.session_depth.is_none(),
        "the last view's depth is the frame's, and `end` has handed the frame back"
    );
}

/// An importer that keeps the last image it wrapped, for a test that reads the picture back out of it.
#[derive(Default)]
struct Keeping {
    last: std::cell::RefCell<Option<wgpu::Texture>>,
}

impl Import for Keeping {
    type Image = u32;
    type Depth = ();

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let texture = Plain.texture(device, meta, image)?;
        *self.last.borrow_mut() = Some(texture.clone());
        Some(texture)
    }
}

/// One texel of a texture this test wrapped, as `[r, g, b]`.
///
/// A whole row is copied even for one texel, because `bytes_per_row` has to satisfy wgpu's alignment.
fn pixel_at(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    x: u32,
    y: u32,
) -> [u8; 3] {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("viewport readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
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

#[test]
fn an_eye_is_drawn_into_its_viewport() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let mut session = session_at(wxr::Extent2d::new(64, 64), false);
    let space = session.space(wxr::SpaceKind::LocalFloor).expect("a floor");
    let mut frame = wxr::Frame::default();
    session.begin(Duration::ZERO, &mut frame).expect("a frame");
    session.views(space, &mut frame).expect("views");
    // A quarter of the image, which is what a runtime that pools two eyes into one texture says each of them
    // owns. The rest of the layer is nobody's, and a pass that drew across all of it would put the eye in the
    // middle instead.
    frame.views_mut()[0].viewport = wxr::Viewport {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
    };

    let mut renderer = Renderer::with_scene(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        [0.0, 0.0, 0.0, 1.0],
        Depth::ZeroToOne,
    );
    let keeping = Keeping::default();
    let drawn = renderer
        .draw(&device, &queue, &mut session, &keeping, &mut frame)
        .expect("the frame draws");
    assert_eq!(drawn, 1);
    let texture = keeping
        .last
        .borrow()
        .as_ref()
        .expect("the importer kept the image")
        .clone();

    // The near triangle covers the centre of whatever viewport it is given, so (16, 16) is inside this one and
    // (48, 48) is outside it. A renderer that ignored the viewport would draw at (32, 32), which leaves the
    // first pixel as the clear and makes the missing viewport a black eye rather than a wrong one.
    assert_ne!(
        pixel_at(&texture, &device, &queue, 16, 16),
        [0, 0, 0],
        "the eye is drawn in the viewport"
    );
    assert_eq!(
        pixel_at(&texture, &device, &queue, 48, 48),
        [0, 0, 0],
        "and nothing is drawn outside it"
    );
}
