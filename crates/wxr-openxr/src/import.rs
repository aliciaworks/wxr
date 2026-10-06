//! Wrapping the compositor's images as wgpu textures.
//!
//! This is the one place a `VkImage` and a `wgpu::Texture` meet, and it is in the backend rather than the
//! renderer for a reason: the renderer is generic over [`wxr::Session::Image`] and cannot name a `VkImage`,
//! and the core does not know what a graphics API is at all. The backend knows both - it is the thing that
//! asked OpenXR for the images - so the `impl` lives here, and neither of the other two has to depend on
//! the other's vocabulary.
//!
//! What the two halves are is worth being exact about. `texture_from_raw` is wgpu-hal being told "this
//! image already exists, do not destroy it"; `create_texture_from_hal` is wgpu being told the same thing in
//! its own terms. Neither makes an image, neither frees one, and the session has to outlive both - which is
//! the caller's contract, and the reason an OpenXR session is not something a renderer can outlive.

use ash::vk::Handle as _;

use super::OpenXrSession;

impl wxr_render::Import for OpenXrSession {
    type Image = u64;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let format = wxr_render::texture_format(meta.format)?;
        let size = wgpu::Extent3d {
            width: meta.extent.width,
            height: meta.extent.height,
            depth_or_array_layers: meta.layers,
        };

        // SAFETY: `image` is one of this session's compositor images, which exist for as long as the session
        // and were created at the size and format asked for above.
        let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }?;
        let descriptor = wgpu::hal::TextureDescriptor {
            label: Some("openxr compositor image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUses::COLOR_TARGET,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: Vec::new(),
        };
        // SAFETY: the image is the runtime's and stays valid while the session does, which is longer than
        // this texture is meant to live. `None` for the drop callback is what says "do not free it": the
        // compositor owns it and will want it back.
        let hal_texture = unsafe {
            hal_device.texture_from_raw(
                ash::vk::Image::from_raw(*image),
                &descriptor,
                None,
                wgpu::hal::vulkan::TextureMemory::External,
            )
        };

        // SAFETY: the hal texture was just made from this device, and the wgpu descriptor says the same
        // things the hal one did - which is what `create_texture_from_hal` requires of it.
        Some(unsafe {
            device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(
                hal_texture,
                &wgpu::TextureDescriptor {
                    label: Some("openxr compositor image"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                // The image arrived from a compositor that has not told us what state it is in, and
                // `UNINITIALIZED` is wgpu's way of saying "transition it yourself, the contents are mine to
                // discard" - which is true: every frame clears it.
                wgpu::TextureUses::UNINITIALIZED,
            )
        })
    }
}
