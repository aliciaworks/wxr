//! Wrapping the browser's textures as wgpu ones.
//!
//! This is the third of the same piece: `wxr-openxr` hands a `VkImage` to `Device::texture_from_raw`, the
//! Apple backend hands an `MTLTexture` to the same function, and here a `GPUTexture` that the *browser* made
//! goes to `Device::create_texture_from_webgpu_handle` - the WebGPU counterpart of those two, with the same
//! contract: it must be the same device, the descriptor must say what the texture really is, and the handle is
//! external, so wgpu never destroys it.
//!
//! The same device is the part that matters and the part that is not this crate's to arrange: a WebXR session
//! hands out textures made by *its* device, so the renderer's device has to be that one - which is a device
//! made from an XR-compatible adapter, which is the `xr_compatible` field this workspace's wgpu fork carries.
//!
//! The textures arrive as `JsValue`s and become wgpu's own `GpuTexture` without copying anything: they are the
//! same JavaScript objects, and the two bindings for them are the browser's and wgpu's.

use wasm_bindgen::JsCast;

/// The browser's images, as the renderer's importer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Images;

impl wxr_render::Import for Images {
    /// The browser's `GPUTexture`s, which are the same objects wgpu names `GpuTexture`.
    type Image = super::FrameImage;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let format = wxr_render::texture_format(meta.format)?;
        wrap(device, format, meta, image.color.clone())
    }

    /// The depth buffer that came with the layer, which is the one the compositor reprojects with.
    ///
    /// It is there because this session asked for it: a projection layer made without a `depthStencilFormat`
    /// has no depth texture, and then this answers `None` and the renderer keeps its own. Asking for the format
    /// the renderer's pipeline was built for is what makes the browser's depth usable rather than a fallback.
    fn depth(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let depth = image.depth.clone()?;
        wrap(device, wxr_render::DEPTH_FORMAT, meta, depth)
    }
}

/// One browser texture as a wgpu one, with the descriptor saying what it really is.
///
/// No drop callback: wgpu never destroys a handle it was given, because the session owns it - and the session
/// outlives every frame that draws into it.
fn wrap(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    meta: wxr::ImageMeta,
    texture: wasm_bindgen::JsValue,
) -> Option<wgpu::Texture> {
    let texture: wgpu::webgpu::GpuTexture = texture.unchecked_into();
    Some(device.create_texture_from_webgpu_handle(
        texture,
        &wgpu::TextureDescriptor {
            label: Some("webxr sub-image"),
            size: wgpu::Extent3d {
                width: meta.extent.width,
                height: meta.extent.height,
                depth_or_array_layers: meta.layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        None,
    ))
}
