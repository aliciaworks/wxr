//! The scaler, end to end on a machine that has one.
//!
//! The device module is what a headset build runs, and macOS links the same framework against the same
//! drivers, so this is the closest thing to a headset that a machine without one has. It is not built for the
//! simulator, which has no MetalFX at all.

use super::SpatialScaler;

const SMALL: u32 = 64;
const LARGE: u32 = 128;

#[test]
fn a_small_picture_comes_back_bigger() {
    let Some((device, queue)) = open() else {
        return;
    };

    // A mid grey rather than black: an upscale of black is black, and a test that cannot tell "wrote
    // nothing" from "wrote what it was given" is not a test.
    let input = texture(&device, SMALL, true);
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &input.create_view(&wgpu::TextureViewDescriptor::default()),
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    queue.submit([encoder.finish()]);
    assert!(
        read_back(&device, &queue, &input, SMALL)[0] > 0x20,
        "the input is not grey"
    );
    let target = texture(&device, LARGE, false);
    // wgpu zero-initialises a texture the first time *it* sees it used, and a MetalFX command buffer is
    // invisible to that bookkeeping - so without this the readback at the end would be wgpu's zeros rather
    // than what the blit wrote. One wgpu pass is enough to make the texture "initialised" as far as wgpu
    // knows, and on a headset it is the compositor reading the image, not wgpu.
    {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.create_view(&wgpu::TextureViewDescriptor::default()),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        queue.submit([encoder.finish()]);
    }

    let Some(scaler) = SpatialScaler::new(
        &device,
        [SMALL, SMALL],
        [LARGE, LARGE],
        wgpu::TextureFormat::Bgra8Unorm,
    ) else {
        // A Mac without MetalFX is not this test's question; the caller's fallback is.
        return;
    };
    assert!(scaler.encode(&queue, &input, &target, 0, [0, 0]));

    let read = read_back(&device, &queue, &target, LARGE);
    assert!(
        read.chunks(4).any(|texel| texel[0] > 0x20),
        "the upscale wrote nothing"
    );
}

fn open() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

fn texture(device: &wgpu::Device, size: u32, clear: bool) -> wgpu::Texture {
    let _ = clear;
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC
        | wgpu::TextureUsages::RENDER_ATTACHMENT;
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("metalfx"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage,
        view_formats: &[],
    })
}

fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: u32,
) -> Vec<u8> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (size * size * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("the device finished");
    slice.get_mapped_range().expect("mapped").to_vec()
}
