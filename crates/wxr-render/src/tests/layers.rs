//! A layer's picture, drawn once into the image the compositor handed over.
//!
//! A layer is not a frame's eye: the compositor places it and warps it for the optics, so it is drawn once,
//! with no [`wxr::View`] to draw it with, and none of the frame loop reaches it. `examples/layers.rs` makes a
//! layer and hands it back, and that example draws nothing into it - which left [`Renderer::draw_layer`], a
//! public method on the renderer, with no caller and nothing that had ever run it. These two tests are that
//! caller.

use super::*;

/// An importer that keeps the last texture it wrapped, so a test can read back out of it.
///
/// `draw_layer` makes the render target through the importer and then drops it, so without this the picture
/// a layer drew is gone by the time the test could look at it. Keeping it is the whole difference between
/// "the call returned true" and "the clear reached the image".
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
        _image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        // Its own texture rather than `Plain`'s, because the whole point of this importer is that the test
        // reads back out of it afterwards - and a texture a compositor handed over would carry only
        // `RENDER_ATTACHMENT`, where reading one back wants `COPY_SRC` as well. The format matches what
        // `Plain` makes so that the picture is the same one a real backend would produce.
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kept layer"),
            size: wgpu::Extent3d {
                width: meta.extent.width,
                height: meta.extent.height,
                depth_or_array_layers: meta.layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        *self.last.borrow_mut() = Some(texture.clone());
        Some(texture)
    }
}

/// An importer with nothing to wrap, which is a browser whose session handed over no image.
struct Refused;

impl Import for Refused {
    type Image = u32;
    type Depth = ();

    fn texture(
        &self,
        _device: &wgpu::Device,
        _meta: ImageMeta,
        _image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        None
    }
}

/// The one texel at the centre of a texture this test wrapped, as `[r, g, b]`.
///
/// A whole row has to be copied even for one texel, because `bytes_per_row` has to satisfy wgpu's alignment
/// - so the buffer is a row wide and the middle texel of it is read.
fn centre_of(texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) -> [u8; 3] {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("layer readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: texture.width() / 2,
                y: texture.height() / 2,
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

/// A layer is drawn once, and what the pass did is in the image it was given.
///
/// Two things are asserted rather than one, and neither stands for the other: the callback running says the
/// renderer handed a real pass to it, and the pixel says what that pass left behind. A method that built a
/// pass and dropped it before submitting would satisfy the first and fail the second, which is the failure
/// that would otherwise reach a compositor as a black panel.
#[test]
fn a_layers_picture_is_left_in_the_image_it_was_given() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };

    let meta = ImageMeta {
        format: wxr::ColorFormat::Rgba8Unorm,
        extent: wxr::Extent2d {
            width: 8,
            height: 8,
        },
        layers: 1,
    };
    let layer = wxr::LayerImage {
        meta,
        // The whole image, which is what a platform that gives a layer a texture of its own hands over.
        viewport: wxr::Viewport {
            x: 0,
            y: 0,
            width: meta.extent.width,
            height: meta.extent.height,
        },
    };

    // Pure green, and chosen for the value rather than the colour: every channel is either zero or one, and
    // those are the two values a linear and an sRGB reading of them agree on. The texture is sRGB and the
    // readback is its encoded bytes, so any other colour would be asserting a conversion this test is not
    // about.
    let mut renderer = Renderer::new([0.0, 1.0, 0.0, 1.0]);
    let keeping = Keeping::default();
    let ran = std::cell::Cell::new(false);
    let drawn = renderer.draw_layer(&device, &queue, &keeping, &0, layer, |pass| {
        ran.set(true);
        // A pass state that needs no pipeline, so the callback is doing something to a real pass rather than
        // only being given one.
        pass.set_blend_constant(wgpu::Color::TRANSPARENT);
    });

    assert!(drawn, "the importer wrapped the image, so there is a layer");
    assert!(
        ran.get(),
        "the callback ran against the pass the renderer made"
    );

    let texture = keeping
        .last
        .borrow()
        .as_ref()
        .expect("the importer kept the texture it wrapped")
        .clone();
    assert_eq!(
        centre_of(&texture, &device, &queue),
        [0, 255, 0],
        "the clear the renderer was given is what a layer starts as",
    );
}

/// An image the importer cannot wrap is a layer that was not drawn, and not a failure.
///
/// This is the answer the eyes already get, and it is the reason the method returns a bool: a browser whose
/// session handed over no image has not broken anything a renderer can fix, so the caller is told there is
/// no layer rather than given an error it would have to invent a recovery for.
#[test]
fn an_image_nothing_can_wrap_is_a_layer_that_was_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };

    let meta = ImageMeta {
        format: wxr::ColorFormat::Rgba8Unorm,
        extent: wxr::Extent2d {
            width: 8,
            height: 8,
        },
        layers: 1,
    };
    let layer = wxr::LayerImage {
        meta,
        viewport: wxr::Viewport {
            x: 0,
            y: 0,
            width: meta.extent.width,
            height: meta.extent.height,
        },
    };

    let mut renderer = Renderer::new([1.0, 0.0, 0.0, 1.0]);
    let ran = std::cell::Cell::new(false);
    let drawn = renderer.draw_layer(&device, &queue, &Refused, &0, layer, |_| ran.set(true));

    assert!(
        !drawn,
        "an image this renderer cannot wrap is a frame with no layer",
    );
    assert!(
        !ran.get(),
        "and the callback is not given a pass that has nothing to draw into",
    );
}

/// A cube is six pictures in one image, and one pass cannot draw them.
///
/// The texture is valid and the importer would wrap it, but a default view of six array layers is not a
/// two-dimensional colour attachment - so the layer is refused before the importer is asked for anything.
/// What would otherwise happen is a validation error, or a cube with one drawn face and five the compositor
/// had to make up.
#[test]
fn a_layered_image_is_not_one_picture() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };

    let meta = ImageMeta {
        format: wxr::ColorFormat::Rgba8Unorm,
        extent: wxr::Extent2d {
            width: 8,
            height: 8,
        },
        layers: 6,
    };
    let layer = wxr::LayerImage {
        meta,
        viewport: wxr::Viewport {
            x: 0,
            y: 0,
            width: meta.extent.width,
            height: meta.extent.height,
        },
    };

    let mut renderer = Renderer::new([1.0, 0.0, 0.0, 1.0]);
    let keeping = Keeping::default();
    let ran = std::cell::Cell::new(false);
    let drawn = renderer.draw_layer(&device, &queue, &keeping, &0, layer, |_| ran.set(true));

    assert!(!drawn, "six faces are six pictures, not one");
    assert!(!ran.get(), "there is no one pass to draw a cube in");
    assert!(
        keeping.last.borrow().is_none(),
        "the image is refused before it is wrapped",
    );
}
