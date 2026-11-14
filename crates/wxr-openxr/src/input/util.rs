//! Small answers the rest of the input code asks: is this tracked, is it a guess, and which way is up.
//!
//! Kept out of `mod.rs` because that file is the actions and the bindings, and these are arithmetic on what a
//! located space says - the kind of thing that grows until the file it was in has no room for the next
//! action.

use openxr as xr;

/// Whether a located space is being tracked.
///
/// Tracked is a question about the position *and* the orientation: a hand behind the user's back has a
/// direction and no place, and a gaze ray the runtime has stopped following is neither.
pub(crate) fn tracked(location: &Option<xr::SpaceLocation>) -> bool {
    location.as_ref().is_some_and(tracked_one)
}

/// Whether one located space is tracked rather than estimated.
pub(crate) fn tracked_one(location: &xr::SpaceLocation) -> bool {
    let flags = location.location_flags;
    flags.contains(xr::SpaceLocationFlags::POSITION_TRACKED)
        && flags.contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED)
}

/// Whether a space located with its velocity is a place the runtime guessed.
///
/// The two halves are separate questions, and this is the position's alone: a three-degree-of-freedom
/// controller has a measured orientation and an estimated place, which is what the core's
/// [`wxr::InputSource::emulated_position`] reports. A location the runtime refused is not an estimate either -
/// there is no pose for the caller to be misled by.
pub(crate) fn estimated(spaced: &Option<(xr::SpaceLocation, xr::SpaceVelocity)>) -> bool {
    spaced.as_ref().is_some_and(|(location, _)| {
        !location
            .location_flags
            .contains(xr::SpaceLocationFlags::POSITION_TRACKED)
    })
}

/// A thumbstick in the core's terms, which is the one axis the two specifications disagree about.
///
/// OpenXR's `y` is positive *down* and the core's [`wxr::Axes`] is positive *up* - the Gamepad API's
/// convention, which is what WebXR reports directly and what the other backend flips. Leaving the sign alone
/// makes every stick drive a camera backwards.
pub(crate) fn thumbstick(value: xr::Vector2f) -> wxr::glam::Vec2 {
    wxr::glam::Vec2::new(value.x, -value.y)
}

/// Whether an analogue button has been pushed far enough to be down.
///
/// A trigger or a grip is a float from 0 to 1 and the core's buttons are digital, so the two need a
/// threshold. Halfway is where every runtime's own input mapping puts it and where a user would say the
/// button was pressed; anything lower makes a resting finger click.
pub(crate) fn pressed(value: f32) -> bool {
    value > 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Down on the hardware is down on the screen: OpenXR's positive Y becomes the core's negative one.
    #[test]
    fn a_thumbstick_y_axis_points_up() {
        let stick = thumbstick(xr::Vector2f { x: 0.5, y: 0.25 });
        assert_eq!(stick, wxr::glam::Vec2::new(0.5, -0.25));
    }

    /// An analogue button is down once it is past half, and a resting trigger is not.
    #[test]
    fn an_analogue_button_is_down_past_half() {
        assert!(!pressed(0.0), "a trigger nobody touched is not pressed");
        assert!(!pressed(0.5), "halfway is not yet a press");
        assert!(pressed(0.6));
        assert!(pressed(1.0));
    }
}
