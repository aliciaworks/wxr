//! One frame: the eyes, where they are, and where they land in the picture.

use std::time::Duration;

use crate::space::{Motion, Pose};

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

/// The two planes a projection is clipped by, which are also what a depth buffer is measured between.
///
/// Nearer than `near` and further than `far` is not drawn, and the pair is the other half of the projection
/// [`FieldOfView`] describes. A compositor needs it for a reason a picture cannot answer: a frame is drawn at
/// a predicted pose and shown at the real one, and reprojecting it means turning the depth buffer back into
/// distance - which takes these two numbers and nothing else in the frame. WebXR calls them
/// `XRRenderState.depthNear` and `depthFar`, OpenXR `XrCompositionLayerDepthInfoKHR`'s `nearZ` and `farZ`, and
/// CompositorServices takes the pair with the **far** one first - which is the reason this is a struct with
/// two names rather than two `f32` arguments in a row.
///
/// One pair for the session rather than one per view, because that is what the three platforms have: WebXR's
/// is on the render state, Apple's on the drawable, and OpenXR's on a layer - which is a view's picture here.
/// A caller that has nothing to say about depth leaves it unset, and a runtime that reprojects without it
/// does what it did before.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DepthRange {
    /// Metres to the near plane, which has to be more than zero for a projection to exist.
    pub near: f32,
    /// Metres to the far plane, past `near`.
    pub far: f32,
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
    /// Whether this is a camera watching the wearer rather than an eye of theirs - WebXR's
    /// `XRView.isFirstPersonObserver`.
    ///
    /// An AR session may show more than the wearer's two eyes: a capture or a spectator view is a third
    /// camera in the same picture, and the specification hides it among the views. What differs is what a
    /// renderer draws into it - the hands a runtime suppresses on the wearer's own eyes, a cursor, a menu
    /// pinned to one eye - so a view that is an observer is one a scene usually draws as a plain camera.
    ///
    /// False by default and false on every view of a headset: an observer view is a camera a phone's AR
    /// session has and a compositor's two eyes are not.
    pub is_first_person_observer: bool,
    /// What scale the runtime suggests rendering this view at, which is WebXR's `recommendedViewportScale`.
    ///
    /// `None` when the runtime has no opinion - and a runtime with no opinion is one where rendering at full
    /// resolution is the right answer, so this is a hint to pass to `Session::request_viewport_scale` rather
    /// than a field to obey.
    pub recommended_viewport_scale: Option<f32>,
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
    /// Where the head is, in whichever reference space the views were asked for - WebXR's
    /// `XRViewerPose.transform`.
    ///
    /// The eyes are placed around it, and a scene that wants the camera *at* the wearer rather than at an eye
    /// asks for this. Filled by `views`, because that is the call the space is given to; identity until then.
    pub viewer: Pose,
    /// Whether [`Frame::viewer`]'s position is estimated rather than tracked - WebXR's
    /// `XRPose.emulatedPosition`, which a `XRViewerPose` carries for the head.
    ///
    /// A 3DOF runtime estimates where the head has moved from what the neck does, and a runtime that has lost
    /// tracking for a moment fills the gap the same way. The pose is still there and still worth drawing with;
    /// what changes is that it is a guess, and content pinned to the room is the thing that shows it. `false`
    /// for a runtime that only reports poses it tracked, which is every one of the three that can say.
    pub emulated_position: bool,
    /// How fast the head is moving, when the runtime tracks that - the same fact `emulated_position` is the
    /// trustworthiness of, on the same pose.
    ///
    /// `None` for a runtime that does not report it, and filled by `views` the way [`Frame::viewer`] is.
    pub motion: Option<Motion>,
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

    /// Forget the last frame: its eyes, where its head was, and how fast it was going.
    ///
    /// A frame is the caller's and is reused, so a runtime that begins a new one has to say that the old
    /// one's views are gone. `viewer` is part of the same fact - it is filled by `views`, and a frame
    /// whose views have not been asked for yet is a frame with no head either; reading last frame's
    /// would stand a scene at last frame's eye height for a frame that never located one. The head's
    /// [`motion`](Frame::motion) and whether its place was a guess are that pose's other halves, so they go
    /// with it.
    pub fn reset(&mut self) {
        self.viewer = Pose::IDENTITY;
        self.emulated_position = false;
        self.motion = None;
        self.views.clear();
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

    #[test]
    fn forgetting_a_frame_forgets_its_eyes_and_its_head() {
        let mut frame = Frame {
            viewer: Pose {
                position: crate::glam::Vec3::Y,
                orientation: crate::glam::Quat::IDENTITY,
            },
            emulated_position: true,
            motion: Some(Motion {
                linear_velocity: Some(crate::glam::Vec3::Z),
                angular_velocity: None,
            }),
            ..Default::default()
        };
        frame.views_mut().push(View {
            eye: Eye::Mono,
            pose: Pose::IDENTITY,
            fov: FieldOfView::symmetric(1.0, 1.0),
            viewport: Viewport::default(),
            image: 0,
            layer: 0,
            is_first_person_observer: false,
            recommended_viewport_scale: None,
        });
        frame.reset();
        assert!(
            frame.views().is_empty(),
            "the old eyes are not this frame's"
        );
        assert_eq!(frame.viewer, Pose::IDENTITY, "and neither is the old head");
        assert!(
            !frame.emulated_position,
            "and nor is the old head's guess still this frame's"
        );
        assert!(
            frame.motion.is_none(),
            "and nor is the speed it was going when it was there"
        );
    }
}
