//! Wrapping the compositor's textures as wgpu ones.
//!
//! This is the same piece `wxr-openxr`'s importer is, the other way up: there, a `VkImage` and the memory
//! behind it were handed to wgpu; here an `MTLTexture`, which owns itself. It is in the backend rather than
//! the renderer for the reason the renderer is generic over [`wxr::Session::Image`] at all - the renderer
//! cannot name either platform's image and the core does not know what a graphics API is, so the only place
//! that can name both ends is the crate that owns one of them.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLTexture;

use super::session::AppleSession;

impl wxr_render::Import for AppleSession {
    type Image = Retained<ProtocolObject<dyn MTLTexture>>;

    fn texture(
        &self,
        device: &wgpu::Device,
        meta: wxr::ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        // SAFETY: `image` is one of the frame's colour textures, which the session holds for as long as the
        // frame is in flight - and the renderer wraps it inside that window. The clone is the wrapper's own
        // reference; wgpu releases it when the texture it made is dropped, which is right, because the
        // compositor's reference is a second one.
        unsafe { super::metal::texture(device, meta, image.clone()) }
    }
}
