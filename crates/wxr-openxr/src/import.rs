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
//!
//! [`Images`] carries nothing, and that is what the renderer's signature leaves room for: see
//! [`wxr_render::Import`]. What it wraps comes in [`SwapchainImage`] pairs, because a frame here is two
//! images - the colour the compositor presents and, when it takes one, the depth it reprojects with.

use ash::vk::Handle as _;

use crate::session::EnvironmentDepthImage;

/// What one frame of the compositor's is, as this backend names it.
///
/// Two images and one frame: the colour the app draws and the compositor presents, and - when the runtime takes
/// the app's depth, which is `XR_KHR_composition_layer_depth` - the depth buffer it reprojects with. Both are
/// `VkImage`s, which on this platform are integers, and both are the *acquired* images rather than the
/// swapchains' first: which one that is changes every frame, and only the acquired one may be drawn into.
///
/// The depth half is `None` on a runtime without the extension, on one that offers no depth format this
/// renderer's pipeline was built for, and on a frame whose depth image could not be taken - and then the
/// renderer makes a private buffer, which loses the reprojection and not the frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct SwapchainImage {
    /// The colour image the app draws into, one array layer per eye.
    pub color: u64,
    /// The depth image the app draws into, one array layer per eye, when there is one to draw into.
    pub depth: Option<u64>,
    /// The motion-vector image the app draws into for space warp, one array layer per eye, when the runtime
    /// hands one over.
    pub motion: Option<u64>,
}

/// The compositor's images, as the renderer's importer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Images;

impl wxr_render::Import for Images {
    type Image = SwapchainImage;
    /// The runtime's own depth map, which is a texture the scene samples rather than a layer to submit.
    type EnvironmentDepth = EnvironmentDepthImage;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let size = wgpu::Extent3d {
            width: meta.extent.width,
            height: meta.extent.height,
            depth_or_array_layers: meta.layers,
        };
        wrap(
            device,
            "openxr compositor image",
            size,
            wxr_render::texture_format(meta.format)?,
            // A layer the runtime composits, and so something the scene draws *into*.
            wgpu::TextureUses::COLOR_TARGET,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            image.color,
        )
    }

    /// The depth the app drew, which the compositor reads to reproject the frame - so a buffer the app
    /// *writes*, the other direction from [`wxr_render::Import::environment_depth`] below.
    ///
    /// The format is the one [`wxr_render::DEPTH_FORMAT`] names, because the depth swapchain is made only
    /// where the runtime offers exactly that: a frame drawn into a buffer of another format is a frame drawn
    /// into a private one, which the compositor never gets to read.
    fn target_depth(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let size = wgpu::Extent3d {
            width: meta.extent.width,
            height: meta.extent.height,
            depth_or_array_layers: meta.layers,
        };
        wrap(
            device,
            "openxr depth image",
            size,
            wxr_render::DEPTH_FORMAT,
            // Read-write, because a pass both tests against it and writes the nearer fragment: this is the
            // buffer the app *draws* its depth into, not one the runtime filled.
            wgpu::TextureUses::DEPTH_STENCIL_WRITE,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            image.depth?,
        )
    }

    /// The motion vectors the app drew, which the compositor synthesises from - `XR_FB_space_warp`'s
    /// per-eye image, drawn the same way the depth is.
    fn target_motion(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let size = wgpu::Extent3d {
            width: meta.extent.width,
            height: meta.extent.height,
            depth_or_array_layers: meta.layers,
        };
        wrap(
            device,
            "openxr motion vectors",
            size,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureUses::COLOR_TARGET,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            image.motion?,
        )
    }

    /// The depth the runtime measured, which is one texture with two array layers - one per eye.
    ///
    /// Sampled and never written to, which is the whole of what this extension is: the runtime measured the
    /// room into it and the scene reads it to decide what is behind what. The format is `VK_FORMAT_D16_UNORM`
    /// and the runtime chose it, so it is named here rather than carried in [`wxr::ImageMeta`], whose
    /// formats are all colour.
    fn environment_depth(
        &self,
        device: &wgpu::Device,
        info: wxr::DepthInfo,
        depth: &Self::EnvironmentDepth,
    ) -> Option<wgpu::Texture> {
        let size = wgpu::Extent3d {
            width: info.size.width,
            height: info.size.height,
            // Two, and not one: the extension makes the image an array of exactly two, and a texture that
            // claimed one layer would be the left eye and no right one.
            depth_or_array_layers: 2,
        };
        wrap(
            device,
            "openxr environment depth",
            size,
            depth_format(depth.format)?,
            // Sampled and never written to: the runtime measured the room into this image and the scene
            // reads it to decide what is behind what.
            wgpu::TextureUses::RESOURCE,
            wgpu::TextureUsages::TEXTURE_BINDING,
            depth.image,
        )
    }
}

/// What a depth format is in wgpu's terms, by its Vulkan number rather than by a magic constant here.
///
/// There is one: the extension mandates `VK_FORMAT_D16_UNORM` for a Vulkan session and the app has no say in
/// it. `None` is a runtime that handed back something else, which the spec does not allow and an importer
/// has no way to use.
fn depth_format(format: u32) -> Option<wgpu::TextureFormat> {
    if format == ash::vk::Format::D16_UNORM.as_raw() as u32 {
        Some(wgpu::TextureFormat::Depth16Unorm)
    } else {
        log::warn!("wxr-openxr: the runtime's depth is not D16_UNORM but {format}");
        None
    }
}

/// Tell wgpu an image that already exists is this texture, in either of its two roles.
///
/// Every one of the importer's images is this one function, because the two halves are the same two halves
/// and the only difference is what the texture is *for*: `texture_from_raw` is wgpu-hal being told "this image
/// already exists, do not destroy it", `create_texture_from_hal` is wgpu being told the same thing in its
/// own terms. Neither makes an image, neither frees one, and the session has to outlive both - which is the
/// caller's contract, and the reason an OpenXR session is not something a renderer can outlive.
fn wrap(
    device: &wgpu::Device,
    label: &str,
    size: wgpu::Extent3d,
    format: wgpu::TextureFormat,
    // What wgpu-hal may do to the image, which is the same set stated its way.
    hal_usage: wgpu::TextureUses,
    // What the texture is for, which is the same set stated wgpu's way.
    usage: wgpu::TextureUsages,
    image: u64,
) -> Option<wgpu::Texture> {
    // SAFETY: `image` is one of this session's own images, which exist for as long as the session and were
    // created at the size and format asked for here.
    let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }?;
    let descriptor = wgpu::hal::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: hal_usage,
        memory_flags: wgpu::hal::MemoryFlags::empty(),
        view_formats: Vec::new(),
    };
    // SAFETY: the image is the runtime's and stays valid while the session does, which is longer than this
    // texture is meant to live. The drop callback is a no-op rather than `None`, and the difference is the
    // whole of the safety: wgpu-hal takes `None` to mean "this image is now mine" and destroys it when the
    // texture goes - which is the compositor's image, freed under it, and the next frame's
    // `vkSetDebugUtilsObjectNameEXT` on the freed handle is a segmentation fault in the driver. A callback
    // that does nothing is what says "the runtime owns it and will want it back".
    let hal_texture = unsafe {
        hal_device.texture_from_raw(
            ash::vk::Image::from_raw(image),
            &descriptor,
            Some(Box::new(|| {})),
            wgpu::hal::vulkan::TextureMemory::External,
        )
    };

    // SAFETY: the hal texture was just made from this device, and the wgpu descriptor says the same things
    // the hal one did - which is what `create_texture_from_hal` requires of it.
    Some(unsafe {
        device.create_texture_from_hal::<wgpu::hal::api::Vulkan>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            },
            // The image arrived from a compositor that has not told us what state it is in, and
            // `UNINITIALIZED` is wgpu's way of saying "transition it yourself, the contents are mine to
            // discard" - which is true: every frame clears it, and the depth map is refilled every frame too.
            wgpu::TextureUses::UNINITIALIZED,
        )
    })
}
