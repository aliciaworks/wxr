//! The Metal half of the importer: a compositor's texture as a wgpu one.
//!
//! This is the same piece as `wxr-openxr`'s importer the other way up. There, a `VkImage` and the memory
//! behind it had to be handed to wgpu; here, an `MTLTexture` - which owns itself - and the two meet
//! `wgpu-hal`'s backends in the same way and for the same reason: the renderer is generic over
//! [`wxr::Session::Image`] and cannot name either, so the conversion lives in the backend that knows one end
//! of it.
//!
//! **Metal 4 does not change this.** wgpu will not adopt Metal 4's command encoders - its own command model
//! is what WebGPU is - and the new encoders are not what this touches. An `MTLTexture` is an `MTLTexture` in
//! every version of Metal; what Metal 4 added is a way to *record* work, and wgpu records its own. So there
//! is nothing to wait for and nothing here that a Metal 4 backend would replace.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLTexture, MTLTextureType};

/// Wrap one of the compositor's textures as the renderer's.
///
/// # Safety
///
/// The texture must outlive the returned one, because it is the compositor's and the compositor will want
/// it back, and it must belong to the device that made it. That is the same contract every `from_raw` in
/// this workspace has.
pub unsafe fn texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    extent: wxr::Extent2d,
    layers: u32,
    raw: Retained<ProtocolObject<dyn MTLTexture>>,
) -> Option<wgpu::Texture> {
    // Checked, not used: a device that is not Metal is a device this cannot wrap for.
    unsafe { device.as_hal::<wgpu::hal::api::Metal>() }?;
    let size = wgpu::Extent3d {
        width: extent.width,
        height: extent.height,
        depth_or_array_layers: layers,
    };

    // SAFETY: the texture is the compositor's and stays valid while the layer does. The retain is this
    // wrapper's own: the hal texture owns it and releases it when wgpu drops the texture, which is correct,
    // because the compositor holds a reference of its own. There is nothing else to release, which is what
    // the empty drop callback says.
    let hal_texture = unsafe {
        wgpu::hal::metal::Device::texture_from_raw(
            raw,
            format,
            // Two eyes are the two array layers of one texture, which is what a compositor hands over.
            if layers > 1 {
                MTLTextureType::Type2DArray
            } else {
                MTLTextureType::Type2D
            },
            layers,
            1,
            wgpu::hal::CopyExtent {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            None,
        )
    };

    // SAFETY: the hal texture was just made from this device, and the wgpu descriptor below says the same
    // things about it - which is what `create_texture_from_hal` requires.
    Some(unsafe {
        device.create_texture_from_hal::<wgpu::hal::api::Metal>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some("compositor image"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            // The compositor has not told us what state the texture is in, and `UNINITIALIZED` is wgpu's way of
            // saying "transition it yourself, the contents are mine to discard" - which is true: every frame
            // clears it.
            wgpu::TextureUses::UNINITIALIZED,
        )
    })
}
