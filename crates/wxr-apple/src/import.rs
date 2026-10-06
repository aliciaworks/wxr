//! Wrapping the compositor's textures as wgpu ones.
//!
//! This is the same piece `wxr-openxr`'s importer is, the other way up: there, a `VkImage` and the memory
//! behind it were handed to wgpu; here an `MTLTexture`, which owns itself. It is in the backend rather than
//! the renderer for the reason the renderer is generic over [`wxr::Session::Image`] at all - the renderer
//! cannot name either platform's image and the core does not know what a graphics API is, so the only place
//! that can name both ends is the crate that owns one of them.
//!
//! [`Images`] is a type with no fields on purpose: see [`wxr_render::Import`] for why an importer is a value
//! of its own rather than the session, which is a constraint the renderer's signature puts on it.

use super::session::FrameImage;

/// The compositor's images, as the renderer's importer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Images;

impl wxr_render::Import for Images {
    /// The colour texture and the depth buffer the drawable gave with it, which are one image here.
    type Image = FrameImage;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        let format = wxr_render::texture_format(meta.format)?;
        // SAFETY: `image` is one of this frame's images, and the session holds it for as long as the frame is
        // in flight - which is the window the renderer wraps it in. The clone is the wrapper's own reference;
        // wgpu releases it when the texture it made is dropped, which is right, because the compositor's
        // reference is a second one.
        unsafe {
            super::metal::texture(
                device,
                format,
                meta.extent,
                meta.layers,
                image.color.clone(),
            )
        }
    }

    /// The drawable's own depth buffer, which is what the compositor reprojects with.
    ///
    /// The same plumbing as the colour one, and for the same reason: a `CompositorServices` drawable gives
    /// both, its depth is `depth32float`, and a renderer that made its own would be submitting a picture
    /// without the depth that goes with it.
    fn depth(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        // SAFETY: as above, for the depth texture the same drawable handed over with the colour one.
        unsafe {
            super::metal::texture(
                device,
                wxr_render::DEPTH_FORMAT,
                meta.extent,
                meta.layers,
                image.depth.clone(),
            )
        }
    }
}
