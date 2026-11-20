//! The trait a backend implements, which is the seam the crate docs describe.
//!
//! In a file of its own because it is the part written on the other side of that seam: an implementation
//! names a platform's images and a `wgpu::Texture` at once, and lives in the backend rather than here.

use wxr::ImageMeta;

/// How a backend's images become wgpu textures.
///
/// One call per image per *frame*, because a compositor's handle may be: a `CompositorServices` drawable's
/// textures and a WebXR sub-image's belong to the frame they came with, and OpenXR's is whichever image the
/// frame acquired. A renderer that kept the first one would draw into a picture the compositor has already
/// shown, so there is no cache and no assumption - what the session hands over now is what is wrapped now.
///
/// An importer is a **value of its own** rather than the session it belongs to, and that is a constraint
/// [`crate::Renderer::draw`] puts on it rather than a style: `draw` needs the session mutably - drawing is
/// presenting - and the importer at the same time, so a backend that implemented this on its session would
/// have an importer it could never pass. A backend writes it on a marker type, which is also what makes it
/// obvious that there is nothing in it: wrapping an image is a function of the image and the device.
pub trait Import {
    /// The session's own name for an image, which is [`wxr::Session::Image`] on the session this is written
    /// for.
    type Image;

    /// The session's own name for the depth it measured of the room - [`wxr::Session::EnvironmentDepth`] on
    /// the session this is written for. `()` for a backend with none, which is a session that measured
    /// nothing. The *other* depth here is the attachment a pass draws into, and it needs no type: it comes
    /// with the image and is the same texture the compositor would otherwise make for the app.
    type EnvironmentDepth;

    /// Wrap one of the session's images.
    ///
    /// `None` when it cannot be - a browser whose WebXR session has no images at all, an image whose format
    /// this renderer does not know - because a frame that cannot be drawn is not an error the renderer can do
    /// anything about.
    fn texture(
        &self,
        device: &wgpu::Device,
        meta: ImageMeta,
        image: &Self::Image,
    ) -> Option<wgpu::Texture>;

    /// The depth attachment this frame's image carries, if the session's compositor takes one.
    ///
    /// It is the *target*: the buffer a pass is drawn into, which the compositor then reads to reproject the
    /// frame - move it to match where the head really is when the picture reaches the display. Two of the
    /// three backends hand one over with every image: a `CompositorServices` drawable has a depth texture, and
    /// a WebXR/WebGPU sub-image has a `depthStencilTexture`. Drawing into the compositor's own is the whole
    /// difference between submitting depth and keeping it, and a frame drawn into a private one still reaches
    /// the display, just without the reprojection.
    ///
    /// `None` - the default - is a session whose images carry no depth, and then the renderer makes its own.
    /// So is an attachment of a format this renderer's pipeline was not built for: the renderer decides that
    /// per view, and falling back is deliberate.
    fn target_depth(
        &self,
        _device: &wgpu::Device,
        _meta: ImageMeta,
        _image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        None
    }

    /// The motion vectors the app drew, which the compositor synthesises intermediate frames from -
    /// `XR_FB_space_warp`'s per-eye image, the other half of the pair with `target_depth`.
    ///
    /// Two components and no more, in metres of screen space rather than in texels: how a vector is encoded
    /// is the renderer's and the runtime's agreement, and the extension's convention is the only one it
    /// takes. `None` - the default - is a session that does not synthesise, and a renderer that asked would
    /// be drawing into nothing.
    fn target_motion(
        &self,
        _device: &wgpu::Device,
        _meta: wxr::ImageMeta,
        _image: &Self::Image,
    ) -> Option<wgpu::Texture> {
        None
    }

    /// The room's depth for a view, as a texture to test fragments against, when the session measured one.
    ///
    /// [`Import::target_depth`] above is the buffer a pass is *drawn into*; this is the measurement of the real
    /// world beside it, and it is what a scene throws occluded fragments away against. The metadata comes
    /// along because an importer needs the buffer's size to describe it, and a shader needs what its values
    /// mean.
    ///
    /// `None` - the default - is a session with no depth to test against, and then the scene draws everything.
    fn environment_depth(
        &self,
        _device: &wgpu::Device,
        _info: wxr::DepthInfo,
        _depth: &Self::EnvironmentDepth,
    ) -> Option<wgpu::Texture> {
        None
    }
}
