//! The compositor's half of the Apple backend: the layer, the frames, and the two eyes.
//!
//! `cp_layer_renderer_query_next_frame` hands out a frame, the frame hands out a drawable, the drawable
//! hands out one colour texture per image and one view per eye, and the view's texture map says which part
//! of which texture that eye goes in. That is the whole of the frame loop, and all of it is C.
//!
//! The eyes themselves are not. `Drawable.View.transform` and `.tangents` are Swift properties with no C
//! accessor, so [`EyeView`] is what the shim reads them into, and [`AppleSession::set_eyes`] is where it
//! hands them over. Everything below that is the same in either language: a transform is a pose the other
//! way round and tangents are angles waiting for an `atan`, which is why both of those are in
//! [`wxr_render`] and tested there rather than here.
//!
//! **What this cannot finish is the present.** Apple encodes the presentation event into the command buffer
//! that drew the frame - `cp_drawable_encode_present` takes an `MTLCommandBuffer` and aborts if it has
//! already been committed - and that buffer is wgpu's, made inside `Queue::submit` where a backend has no
//! reach. Every other part of the frame is here and the gap is one call wide; closing it means either a
//! wgpu hook that runs before the submit commits or the Metal 4 queue path, and it is written down instead
//! of guessed at. `end` does the rest of the frame's envelope.

use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_compositor_services::{
    cp_drawable, cp_drawable_array, cp_drawable_t, cp_drawable_target, cp_frame, cp_frame_t,
    cp_frame_timing, cp_layer_renderer_get_state, cp_layer_renderer_query_next_frame,
    cp_layer_renderer_state, cp_layer_renderer_t, cp_time, cp_view, cp_view_texture_map,
};
use objc2_metal::{MTLDevice, MTLPixelFormat, MTLTexture};

use wxr::glam::Mat4;

/// One eye, as the Swift half of CompositorServices sees it.
///
/// This is [`EyeView::transform`] and [`EyeView::tangents`] of `LayerRenderer.Drawable.View`, which Apple
/// documents as Swift properties and provides no C accessor for. The shim fills one of these per view, once
/// a frame, and calls [`AppleSession::set_eyes`] before the frame's views are asked for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EyeView {
    /// Device space to the eye, column-major, which is how a `simd_float4x4` is laid out.
    pub transform: [f32; 16],
    /// The tangents of the four half-angles, in the order CompositorServices gives them: left, right, up,
    /// down.
    pub tangents: [f32; 4],
}

/// The layer renderer, which is everything a session is made from.
///
/// The layer is made by the app, in Swift: an immersive space presented with `CompositorLayer` hands the
/// renderer to a closure, and the closure is where Rust is entered. So there is nothing to look up and
/// nothing to load - the platform has already decided there is a display, which is why this backend has no
/// two-step of the kind OpenXR needs.
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
    /// renderer hands to [`connect`](wxr::Backend::connect) is nothing.
    pub fn device(&self) -> Retained<ProtocolObject<dyn MTLDevice>> {
        // SAFETY: the layer renderer is live and this crate holds a reference to it.
        unsafe { objc2_compositor_services::cp_layer_renderer_get_device(&self.renderer) }
    }
}

impl wxr::Backend for AppleBackend {
    /// Nothing: the compositor owns the device, so there are no handles to hand over. See
    /// [`AppleBackend::device`].
    type Device = ();
    type Session = AppleSession;

    fn connect(&self, _device: ()) -> Result<AppleSession, wxr::Error> {
        Ok(AppleSession::new(self.renderer.clone()))
    }
}

/// A live session, with a compositor presenting what the renderer draws.
pub struct AppleSession {
    renderer: Retained<cp_layer_renderer_t>,
    /// The frame the layer handed out and the drawable in it, for as long as they last. Both are the
    /// layer's and are given back in `end`.
    frame: cp_frame_t,
    drawable: cp_drawable_t,
    /// The frame's colour textures, retained while the frame is in flight: the drawable's own reference is
    /// not ours to keep, and the renderer wraps these between `begin` and `end`.
    textures: Vec<Retained<ProtocolObject<dyn MTLTexture>>>,
    /// Where the eyes are, which arrives from the shim.
    eyes: Vec<EyeView>,
    predicted: Duration,
    /// The last state `poll` reported, so that a state the layer is simply still in is not news twice.
    reported: wxr::State,
    spaces: u32,
}

impl AppleSession {
    fn new(renderer: Retained<cp_layer_renderer_t>) -> Self {
        Self {
            renderer,
            frame: std::ptr::null_mut(),
            drawable: std::ptr::null_mut(),
            textures: Vec::new(),
            eyes: Vec::new(),
            predicted: Duration::ZERO,
            reported: wxr::State::Synchronized,
            spaces: 0,
        }
    }

    /// Tell the session where the eyes are this frame.
    ///
    /// Which is what the Swift half of CompositorServices knows and the C half does not - see [`EyeView`].
    /// One entry per view, in the drawable's order.
    pub fn set_eyes(&mut self, eyes: &[EyeView]) {
        self.eyes.clear();
        self.eyes.extend_from_slice(eyes);
    }

    /// What the layer is doing, which is the one thing it reports.
    ///
    /// There is no event queue on this side - the C surface has a state and no events - so this is read
    /// rather than waited for.
    fn layer_state(&self) -> wxr::State {
        // SAFETY: the layer renderer is live.
        let state = unsafe { cp_layer_renderer_get_state(&self.renderer) };
        if state == cp_layer_renderer_state::running {
            // The layer is ready for a frame. `Focused` would say the person is in the session rather than
            // looking at it, and the compositor does not report that, so this is where the ladder stops.
            wxr::State::Visible
        } else if state == cp_layer_renderer_state::invalidated {
            wxr::State::Ended
        } else {
            // `paused`: the layer exists and is not drawing. A session waiting to be shown.
            wxr::State::Synchronized
        }
    }

    /// Give the frame back. The textures are the compositor's and were only borrowed for one frame.
    fn release(&mut self) {
        self.textures.clear();
        self.drawable = std::ptr::null_mut();
        self.frame = std::ptr::null_mut();
    }
}

impl wxr::Session for AppleSession {
    type Image = Retained<ProtocolObject<dyn MTLTexture>>;

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.layer_state()
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        let state = self.layer_state();
        if state == self.reported {
            return None;
        }
        self.reported = state;
        Some(wxr::Event::StateChanged(state))
    }

    fn images(&self) -> wxr::ImageMeta {
        // The texture is the truth about itself. The layer's configuration is what the app *asked* for, and
        // the compositor is free to hand back something else - so the format, the size and the layer count
        // are read off the thing that will actually be drawn into.
        let Some(texture) = self.textures.first() else {
            return wxr::ImageMeta::default();
        };
        let (format, width, height, layers) = (
            texture.pixelFormat(),
            texture.width() as u32,
            texture.height() as u32,
            texture.arrayLength() as u32,
        );
        wxr::ImageMeta {
            format: color_format(format),
            extent: wxr::Extent2d::new(width, height),
            layers,
        }
    }

    fn image_count(&self) -> usize {
        // A compositor may dedicate a texture to each view or layer the views in one; the drawable's
        // texture count is the number either way, and a view's texture map is what says which is which.
        self.textures.len()
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        self.textures.get(index)
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        // The only space a compositor knows on its own is the device's: where each eye is relative to the
        // wearer. A floor is the tracking half's to know, and the tracking half is the shim's - so a scene
        // that asks to stand on one is told no rather than put at eye height.
        if kind != wxr::SpaceKind::Viewer {
            return Err(wxr::Error::NoSpace(kind));
        }
        self.spaces += 1;
        Ok(wxr::ReferenceSpace::new(kind, self.spaces))
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.release();
        out.views_mut().clear();
        out.state = wxr::FrameState::Wait;

        // The layer has a frame when it has one, and `nil` when it does not: paused, invalidated, or a frame
        // already in flight. Waiting is the right answer to all three, and none of them is an error.
        // SAFETY: the layer renderer is live, and a frame it returns is used until `end` gives it back.
        self.frame = unsafe { cp_layer_renderer_query_next_frame(&self.renderer) };
        if self.frame.is_null() {
            return Ok(());
        }

        // SAFETY: the frame was just handed out and has not been given back.
        self.drawable = unsafe {
            let array = cp_frame::query_drawables(self.frame);
            if array.is_null() {
                std::ptr::null_mut()
            } else {
                // The array holds one drawable per display target - what the wearer sees, and a capture one
                // for streaming - and it is the built-in one that a person is looking at.
                (0..cp_drawable_array::count(array))
                    .map(|index| cp_drawable_array::drawable(array, index))
                    .find(|drawable| {
                        !drawable.is_null()
                            && cp_drawable::target(*drawable) == cp_drawable_target::built_in
                    })
                    .unwrap_or(std::ptr::null_mut())
            }
        };
        if self.drawable.is_null() {
            self.release();
            return Ok(());
        }

        // SAFETY: the frame is the layer's, it is not in flight twice, and it is read between this
        // `start_update` and the `end_update` that `views` performs.
        unsafe {
            cp_frame::start_update(self.frame);
            let timing = cp_frame::predict_timing(self.frame);
            // `now` is not used: the compositor's clock is a Mach one and comparing it against a wall clock
            // is a comparison between two unrelated epochs. What it predicts is what is wanted anyway.
            self.predicted = Duration::from_secs_f64(
                cp_time::to_cf_time_interval(cp_frame_timing::presentation_time(timing)).max(0.0),
            );
            for index in 0..cp_drawable::texture_count(self.drawable) {
                self.textures
                    .push(cp_drawable::color_texture(self.drawable, index));
            }
        }

        out.predicted_display_time = self.predicted;
        out.state = wxr::FrameState::Render;
        Ok(())
    }

    fn views(
        &mut self,
        _space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        if self.drawable.is_null() {
            return Ok(());
        }
        // SAFETY: the drawable is this frame's and is live until `end`. Every index below is below the view
        // count just read, and the texture map lives as long as the view it came from.
        let (count, maps) = unsafe {
            let count = cp_drawable::view_count(self.drawable);
            let maps = (0..count)
                .map(|index| {
                    let view = cp_drawable::view(self.drawable, index);
                    let map = cp_view::view_texture_map(view);
                    (
                        cp_view_texture_map::texture_index(map),
                        cp_view_texture_map::slice_index(map) as u32,
                        cp_view_texture_map::viewport(map),
                    )
                })
                .collect::<Vec<_>>();
            (count, maps)
        };

        for (index, (image, layer, viewport)) in maps.into_iter().enumerate() {
            // A view with no eye to go with it is a view the shim has not described. Dropping it is better
            // than drawing it at the origin: the frame is still presentable with the eyes that were named.
            let Some(eye) = self.eyes.get(index).copied() else {
                log::warn!(
                    "wxr-apple: view {index} of {count} has no transform; the shim described {} eyes",
                    self.eyes.len()
                );
                break;
            };
            out.views_mut().push(wxr::View {
                eye: if count == 1 {
                    wxr::Eye::Mono
                } else if index == 0 {
                    wxr::Eye::Left
                } else {
                    wxr::Eye::Right
                },
                pose: wxr_render::pose_from_view(Mat4::from_cols_array(&eye.transform)),
                fov: wxr_render::angles_from_tangents(eye.tangents),
                viewport: wxr::Viewport {
                    x: viewport.originX.max(0.0) as u32,
                    y: viewport.originY.max(0.0) as u32,
                    width: viewport.width.max(0.0) as u32,
                    height: viewport.height.max(0.0) as u32,
                },
                image,
                layer,
            });
        }

        // The queries are finished; rendering starts after this and the frame is submitted in `end`.
        // SAFETY: the frame is the one begun above and has not been given back.
        unsafe { cp_frame::end_update(self.frame) };
        Ok(())
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        if self.frame.is_null() {
            return Ok(());
        }
        // The submission envelope, and not the presentation event: that one is encoded into the command
        // buffer that drew the frame, which wgpu owns. See the module comment - this is the one call wide
        // gap in the backend, and it is named rather than hidden.
        // SAFETY: the frame is the one begun above and has not been given back.
        unsafe {
            cp_frame::start_submission(self.frame);
            cp_frame::end_submission(self.frame);
        }
        self.release();
        Ok(())
    }
}

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
