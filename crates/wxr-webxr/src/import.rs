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
//! The texture arrives as a `JsValue` and becomes wgpu's own `GpuTexture` without copying anything: it is the
//! same JavaScript object, and the two bindings for it are the browser's and wgpu's.

use wasm_bindgen::JsCast;

/// The browser's images, as the renderer's importer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Images;

impl wxr_render::Import for Images {
    /// The browser's `GPUTexture`, which is the same object wgpu names `GpuTexture`.
    type Image = wasm_bindgen::JsValue;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let format = wxr_render::texture_format(meta.format)?;
        let texture: wgpu::webgpu::GpuTexture = image.clone().unchecked_into();
        // No drop callback: wgpu never destroys a handle it was given, because the session owns it - and the
        // session outlives every frame that draws into it.
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
}
