//! One frame: the eyes, where they are, and where they land in the picture.

use std::time::Duration;

use crate::space::Pose;

/// Which eye a view is for.
///
/// A runtime that presents one image has [`Eye::Mono`], and it is not a degenerate case to be folded into
/// "left": a phone held up is a real XR session with one eye, and a renderer that assumed two would have to
/// invent a second pose for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Eye {
    Mono,
    Left,
    Right,
}

/// A field of view, as the four directions it opens in.
///
/// Asymmetric on purpose. A headset's lens is not centred on the panel, and a runtime reports what the
/// optics actually are rather than a symmetric approximation of them - an approximation that would put the
/// projection centre where the panel centre is and skew everything the user sees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FieldOfView {
    /// Radians above the centre.
    pub up: f32,
    /// Radians below it.
    pub down: f32,
    /// Radians to the left, from the centre of projection.
    pub left: f32,
    /// Radians to the right.
    pub right: f32,
}

impl FieldOfView {
    /// A field that opens the same amount in every direction.
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            up: vertical * 0.5,
            down: vertical * 0.5,
            left: horizontal * 0.5,
            right: horizontal * 0.5,
        }
    }

    pub fn width(self) -> f32 {
        self.left + self.right
    }

    pub fn height(self) -> f32 {
        self.up + self.down
    }

    pub fn aspect(self) -> f32 {
        self.width() / self.height()
    }

    /// How far off-centre the projection is, horizontally, as a fraction of the width.
    pub fn offset_x(self) -> f32 {
        (self.right - self.left) / self.width()
    }
}

/// Where a view is drawn in the compositor's image.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// One eye's worth of a frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct View {
    pub eye: Eye,
    /// In whichever [`crate::space::ReferenceSpace`] the views were asked for.
    pub pose: Pose,
    pub fov: FieldOfView,
    /// Where in the image this eye is drawn - a sub-rectangle, not necessarily a whole layer.
    pub viewport: Viewport,
    /// Which of the session's images, indexed as the backend hands them out.
    pub image: usize,
    /// Which array layer of that image.
    pub layer: u32,
}

/// What the caller should do with the frame it just began.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FrameState {
    /// Draw into the views and present.
    Render,
    /// The session is not showing anything yet. Wait and begin another frame.
    /// The default, because a frame nobody has begun is not a frame to draw.
    #[default]
    Wait,
    /// The runtime wants out. Stop, rather than presenting anything else.
    Exit,
}

/// A frame the runtime has begun.
///
/// The views are owned and reused by the session, so a frame loop does not allocate a `Vec` per frame - and
/// so a caller cannot hold on to one frame's views while the next is being built.
#[derive(Default, Debug)]
pub struct Frame {
    /// When the picture will be on the display, as the runtime predicts it. A simulation that wants to be
    /// in step with the head wants to be sampled for this time and not for now.
    pub predicted_display_time: Duration,
    pub state: FrameState,
    views: Vec<View>,
}

impl Frame {
    pub fn views(&self) -> &[View] {
        &self.views
    }

    /// The views, to be filled by the backend. Emptying it first is the backend's business.
    pub fn views_mut(&mut self) -> &mut Vec<View> {
        &mut self.views
    }

    /// Whether the caller has anything to draw.
    pub fn is_render(&self) -> bool {
        self.state == FrameState::Render
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn a_quarter_turn_is_a_square() {
        let fov = FieldOfView::symmetric(FRAC_PI_2, FRAC_PI_2);
        assert!((fov.aspect() - 1.0).abs() < 1e-6);
        assert!(fov.offset_x().abs() < 1e-6, "symmetric means centred");
    }

    #[test]
    fn an_offset_projection_is_not_centred() {
        // All of the opening on the right: the centre sits on the left edge.
        let fov = FieldOfView {
            up: 0.5,
            down: 0.5,
            left: 0.0,
            right: 1.0,
        };
        assert!((fov.offset_x() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_frame_with_no_runtime_is_not_a_render() {
        let frame = Frame::default();
        assert!(!frame.is_render());
        assert!(frame.views().is_empty());
    }
}
