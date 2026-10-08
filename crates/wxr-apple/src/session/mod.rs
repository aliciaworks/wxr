//! The compositor's half of the Apple backend: the layer, the frames, and the two eyes.
//!
//! `cp_layer_renderer_query_next_frame` hands out a frame, the frame hands out a drawable, the drawable
//! hands out one colour texture per image and one view per eye, and the view says where that eye is
//! (`cp_view_get_transform`), how it opens (`cp_view_get_tangents`) and which part of which texture it goes
//! in (the texture map). That is the whole of the frame loop, and every part of it is C - which is the
//! correction this module is built on: the eye transforms are not a Swift property that needs a shim, they
//! are [`crate::sys::cp_view_get_transform`], and the C API has been there all along.
//!
//! The two things the compositor does not answer are outside it, and both are in the crate:
//!
//! * **Where that is in the world.** Device space is the wearer's head, so a pose in it is a scene that
//!   follows them. [`crate::arkit`] is ARKit's session in C, and it is what makes [`wxr::SpaceKind::Local`]
//!   an answer rather than a refusal - and what fills [`wxr::Session::inputs`], because on this platform a
//!   hand is the input.
//! * **When the picture appears.** `cp_drawable_encode_present` has to be encoded into a command buffer and
//!   committed, and the command buffer the renderer drew with is wgpu's, made and committed inside
//!   `Queue::submit` where no backend can reach it. So the present gets a command buffer of its own on the
//!   same queue - Metal runs command buffers on one queue in commit order, so it sequences behind the frame
//!   that was drawn, which is all the compositor needs. `present` is that call.

use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_compositor_services::{
    cp_axis_direction_convention, cp_drawable, cp_drawable_array, cp_drawable_t,
    cp_drawable_target, cp_frame, cp_frame_t, cp_frame_timing,
    cp_layer_renderer_configuration_get_color_format, cp_layer_renderer_get_configuration,
    cp_layer_renderer_get_state, cp_layer_renderer_query_next_frame, cp_layer_renderer_state,
    cp_layer_renderer_t, cp_time, cp_view, cp_view_texture_map,
};
use objc2_metal::{MTLCommandBuffer, MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLTexture};

use wxr::glam::Mat4;

use crate::arkit::ArKit;
use crate::sys;

/// The layer renderer, which is everything a session is made from.
///
/// The layer is made by the app, in Swift: an immersive space presented with `CompositorLayer` hands the
/// renderer to a closure, and the closure is where Rust is entered. That is three lines of Swift that
/// belong to the app rather than a shim this backend needs - there is nothing to translate in them.
pub struct AppleBackend {
    renderer: Retained<cp_layer_renderer_t>,
}

impl AppleBackend {
    pub fn new(renderer: Retained<cp_layer_renderer_t>) -> Self {
        Self { renderer }
    }

    /// The Metal device the layer draws with, which is the device wgpu must be given.
    ///
    /// This is the core's device story from the other side, and it is worth naming: the renderer is supposed
    /// to make the device and tell the session, but on visionOS the compositor owns the device, because the
    /// textures it hands out belong to one. So the app takes this and makes wgpu adopt it, and what the
    /// renderer hands to [`connect`](wxr::Backend::connect) is the queue it draws on - which the session
    /// needs, because presenting is committing a command buffer on that same queue.
    ///
    /// Handing it to wgpu is a step this crate cannot take for the app, and what is known about it is worth
    /// writing down rather than leaving to be rediscovered. wgpu's Metal backend enumerates devices with
    /// `MTLCopyAllDevices`, so on a device with one - which visionOS is - an ordinary `request_adapter` lands
    /// on this same device and nothing needs adopting. A caller that wants to be certain has
    /// `wgpu-hal`'s `metal::Adapter::new(&device)`, which is public, and then
    /// `Instance::create_adapter_from_hal` - which needs a whole `ExposedAdapter`, and the function that
    /// builds one properly (`AdapterShared::expose`) is private, so its `AdapterInfo` has to be assembled by
    /// hand. That field is for reporting; the device is the part that matters, and it is the one being handed
    /// over here.
    pub fn device(&self) -> Retained<ProtocolObject<dyn MTLDevice>> {
        // SAFETY: the layer renderer is live and this crate holds a reference to it.
        unsafe { objc2_compositor_services::cp_layer_renderer_get_device(&self.renderer) }
    }
}

impl wxr::Backend for AppleBackend {
    /// The renderer's queue. See [`AppleBackend::device`] for why it is not a device.
    type Device = wgpu::Queue;
    type Session = AppleSession;

    /// The immersive space's style is the app's choice in its own language, and `set_blend` is how it reaches
    /// this session afterwards - so there is no mode to ask for here either.
    fn connect(
        &self,
        queue: wgpu::Queue,
        _mode: wxr::SessionMode,
    ) -> Result<AppleSession, wxr::Error> {
        Ok(AppleSession::new(self.renderer.clone(), queue))
    }
}

/// One of the frame's images: the colour texture, and the depth buffer the compositor gave with it.
///
/// They are one thing here because the compositor hands them over as one - both come from the drawable, per
/// texture index - and because that is what lets an importer answer for both. The depth is the drawable's own,
/// not one this renderer made, which is the entire difference between submitting depth and keeping it.
pub struct FrameImage {
    pub color: Retained<ProtocolObject<dyn MTLTexture>>,
    pub depth: Retained<ProtocolObject<dyn MTLTexture>>,
}

/// A live session, with a compositor presenting what the renderer draws.
pub struct AppleSession {
    renderer: Retained<cp_layer_renderer_t>,
    /// The queue the renderer draws on, which is the compositor's device's queue and the one the present is
    /// committed on.
    queue: wgpu::Queue,
    /// The frame the layer handed out and the drawable in it, for as long as they last. Both are the
    /// layer's and are given back in `end`.
    frame: cp_frame_t,
    drawable: cp_drawable_t,
    /// The frame's images, retained while the frame is in flight: the drawable's own references are not ours
    /// to keep, and the renderer wraps these between `begin` and `end`.
    textures: Vec<FrameImage>,
    /// The near and far planes the app draws with, which the compositor is told each frame so that it can
    /// reproject with the depth buffer. `None` until an app says - see `set_depth_range`.
    depth_range: Option<(f32, f32)>,
    /// What the display shows behind the picture, told by the app rather than by the compositor - see
    /// `set_blend`.
    blend: wxr::Blend,
    /// ARKit, when it came up. `None` means every pose is relative to the wearer's head and there are no
    /// hands.
    arkit: Option<ArKit>,
    /// The colour format the layer was configured with.
    ///
    /// Kept because it is answerable before there is a frame, which is what lets a caller make its renderer in
    /// advance instead of in the middle of its first one. A drawable's own texture still wins when there is
    /// one: it is what will actually be drawn into.
    configured: wxr::ColorFormat,
    /// Where the device is in the world at this frame's presentation time, when ARKit answered. The frame's
    /// views are built from it, and it is what was handed to the compositor to reproject against.
    origin: Option<Mat4>,
    predicted: Duration,
    /// What `poll` reported last, on both axes, so that a state the layer is simply still in is not news
    /// twice.
    reported: (wxr::State, wxr::Visibility),
    /// Where each space sits inside the one it was made from, oldest first: a space at a place in the room is
    /// this much transform, and an offset of an offset has to add up.
    spaces: Vec<wxr::Pose>,
    /// How many hands were there the last frame, so that the set of inputs changing is news once.
    hands_seen: usize,
    /// Whether the set of inputs changed since the last poll, waiting to be said.
    inputs_changed: bool,
}

impl AppleSession {
    fn new(renderer: Retained<cp_layer_renderer_t>, queue: wgpu::Queue) -> Self {
        // SAFETY: the layer renderer is live, and the configuration is Apple's to hand out - it is the one the
        // app asked for when it made the immersive space.
        let configured = unsafe {
            color_format(cp_layer_renderer_configuration_get_color_format(
                &cp_layer_renderer_get_configuration(&renderer),
            ))
        };
        Self {
            renderer,
            queue,
            frame: std::ptr::null_mut(),
            drawable: std::ptr::null_mut(),
            textures: Vec::new(),
            depth_range: None,
            blend: wxr::Blend::Opaque,
            // Tracking that will not start is a head-locked scene, not a session that failed.
            arkit: ArKit::new(),
            configured,
            origin: None,
            predicted: Duration::ZERO,
            reported: (wxr::State::Ready, wxr::Visibility::Hidden),
            spaces: Vec::new(),
            hands_seen: 0,
            inputs_changed: false,
        }
    }

    /// Tell the session what the display is showing behind the picture.
    ///
    /// The compositor does not say: whether the space mixes with the room is the app's choice of immersion
    /// style, made in Swift, and nothing in the C surface reports it back. So the app that chose says, and the
    /// default until it does is opaque - a fully immersive space, which is what a `CompositorLayer` is unless
    /// somebody asked for otherwise.
    pub fn set_blend(&mut self, blend: wxr::Blend) {
        self.blend = blend;
    }

    /// Whether ARKit gave this session a world to put things in, as opposed to one that follows the wearer.
    pub fn is_world_tracked(&self) -> bool {
        self.arkit.as_ref().is_some_and(ArKit::is_world_tracked)
    }

    /// The depth convention a pass drawn into this session's images has to use.
    ///
    /// Reverse-Z, because that is the only depth a `CompositorServices` drawable accepts - it is what the
    /// compositor reprojects against and what its own `cp_drawable_compute_projection` produces. It is asked
    /// of the session rather than written into the app because the platform is what knows it, which is the
    /// same reason everything else here is a backend.
    pub fn depth(&self) -> wxr_render::Depth {
        wxr_render::Depth::Reverse
    }

    /// The transform from device space into the space the caller asked for.
    ///
    /// ARKit's origin is the `Local` space and device space is identity otherwise, and a space made by
    /// `offset_space` is that one seen from a pose inside it - so this is where an offset becomes a transform,
    /// and there is exactly one place it has to be right.
    fn space_origin(&self, space: wxr::ReferenceSpace) -> Mat4 {
        let base = match (space.kind, self.origin) {
            (wxr::SpaceKind::Local, Some(origin)) => origin,
            _ => Mat4::IDENTITY,
        };
        let offset = self
            .spaces
            .get(space.id() as usize)
            .copied()
            .unwrap_or_default();
        Mat4::from(offset.transform()).inverse() * base
    }

    /// What the layer is doing, as the two things a session can be: here or not, and shown or not.
    ///
    /// There is no event queue on this side - the C surface has a state and no events - so this is read
    /// rather than waited for. Its three states carry both axes between them: a paused layer is a session that
    /// exists and is not on a display, a running one is both, and an invalidated one is over.
    fn layer_state(&self) -> (wxr::State, wxr::Visibility) {
        // SAFETY: the layer renderer is live.
        let state = unsafe { cp_layer_renderer_get_state(&self.renderer) };
        if state == cp_layer_renderer_state::running {
            (wxr::State::Ready, wxr::Visibility::Visible)
        } else if state == cp_layer_renderer_state::invalidated {
            (wxr::State::Ended, wxr::Visibility::Hidden)
        } else {
            (wxr::State::Ready, wxr::Visibility::Hidden)
        }
    }

    /// Give the frame back. The textures are the compositor's and were only borrowed for one frame.
    fn release(&mut self) {
        self.textures.clear();
        self.origin = None;
        self.drawable = std::ptr::null_mut();
        self.frame = std::ptr::null_mut();
    }

    /// Encode the presentation event and commit it, on a command buffer of its own.
    ///
    /// The event has to go into a command buffer before it is committed, and the one that drew the frame is
    /// wgpu's. A second, empty command buffer on the same queue is what is left, and it is enough: command
    /// buffers on one Metal queue run in commit order, so it sequences behind the frame that was drawn.
    fn present(&mut self) {
        if self.drawable.is_null() {
            return;
        }
        // SAFETY: the queue is live, and a device this session was told about is a Metal one - if it were
        // not, the compositor's textures could not have been wrapped by `metal::texture` either.
        let Some(hal) = (unsafe { self.queue.as_hal::<wgpu::hal::api::Metal>() }) else {
            log::error!("wxr-apple: the queue is not a Metal one; the frame cannot be presented");
            return;
        };
        let Some(buffer) = hal.as_raw().commandBuffer() else {
            log::error!("wxr-apple: the compositor's queue made no command buffer");
            return;
        };
        // SAFETY: the drawable is this frame's and has not been given back.
        unsafe { cp_drawable::encode_present(self.drawable, &buffer) };
        buffer.commit();
    }
}

mod dispatch;

/// A Metal pixel format in the core's terms.
///
/// `Unknown` for a format this core has not learned, which the renderer turns into a frame it does not draw
/// - better than one drawn through the wrong answer about what the bits mean.
fn color_format(format: MTLPixelFormat) -> wxr::ColorFormat {
    match format {
        MTLPixelFormat::BGRA8Unorm_sRGB => wxr::ColorFormat::Bgra8Srgb,
        MTLPixelFormat::BGRA8Unorm => wxr::ColorFormat::Bgra8Unorm,
        MTLPixelFormat::RGBA8Unorm_sRGB => wxr::ColorFormat::Rgba8Srgb,
        MTLPixelFormat::RGBA8Unorm => wxr::ColorFormat::Rgba8Unorm,
        MTLPixelFormat::RGBA16Float => wxr::ColorFormat::Rgba16Float,
        MTLPixelFormat::RGB10A2Unorm => wxr::ColorFormat::Rgb10a2Unorm,
        _ => wxr::ColorFormat::Unknown,
    }
}
