//! Styli: the framework's other device, and the one analogue quantity its tip measures.
//!
//! A spatial stylus is a `GCStylus` rather than a `GCController`: it has no extended gamepad profile, it
//! arrives in a list of its own (`GCStylus::styli`) and not in `GCController::controllers`, and its inputs
//! are the physical input elements of a barrel - a pressure-sensitive tip and two side buttons. Since
//! visionOS 26 it is one of the accessories ARKit tracks, so its place, its motion and its named aim point
//! come from `crate::arkit::accessory` exactly as a pad's do; this file is only what the core's buttons and
//! axes mean on it.
//!
//! **The tip is the selection, and its pressure is the trigger.** A tip against a surface is the click of a
//! tracked pointer, and how hard it is pressed is the one analogue quantity a stylus has - which is what the
//! core's trigger is. The primary side button - the one nearest the tip, and the one Apple says is used to
//! grab - is the squeeze.
//!
//! **The secondary button is not reported.** A spatial stylus has a second side button, and the core has no
//! third button to put it in: [`wxr::Buttons::menu`] is the system's button, and a button that opens a menu
//! when the wearer meant to draw is worse than a button that is absent. It is a name this core would need a
//! new word for, and the honest answer until then is that there are two.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_game_controller::{
    GCButtonElement, GCDevicePhysicalInput, GCDevicePhysicalInputState, GCInputButtonName,
    GCInputStylusPrimaryButton, GCInputStylusTip, GCLinearInput, GCPressedStateInput,
};

/// The core's buttons and axes for a stylus, from the physical input profile it reports.
///
/// A stylus with no profile - one the framework has handed out but not filled in yet - reports nothing, the
/// same way a pad with no extended gamepad does.
pub(super) fn read(
    input: Option<&ProtocolObject<dyn GCDevicePhysicalInput>>,
) -> (wxr::Buttons, wxr::Axes) {
    let Some(input) = input else {
        return (wxr::Buttons::default(), wxr::Axes::default());
    };
    // SAFETY: both names are Apple's own NUL-terminated statics, and this only reads them.
    let (tip, primary) = unsafe {
        (
            press(input, GCInputStylusTip),
            press(input, GCInputStylusPrimaryButton),
        )
    };
    mapped(tip, primary.is_some_and(|press| press.pressed))
}

/// One button's state: whether it is pressed, and how hard when the element measures force.
#[derive(Clone, Copy, Default)]
struct Press {
    pressed: bool,
    /// `0..=1`, which is the range [`wxr::Axes::trigger`] is in.
    value: f32,
}

/// What a stylus's tip and primary button mean to the core.
///
/// Separate from the framework so that it can be tested: a tip that is absent is not a tip at rest, and the
/// difference is the one thing this file can get wrong without a stylus in hand.
fn mapped(tip: Option<Press>, primary: bool) -> (wxr::Buttons, wxr::Axes) {
    let tip = tip.unwrap_or_default();
    (
        wxr::Buttons {
            select: tip.pressed,
            squeeze: primary,
            // The secondary button is deliberately not the menu button - see the module comment.
            menu: false,
        },
        wxr::Axes {
            trigger: tip.value,
            // A stylus has one axis - the tip's - and no stick, so the stick is where it rests.
            thumbstick: wxr::glam::Vec2::ZERO,
        },
    )
}

/// One button of a stylus's profile, if this stylus has it.
///
/// A tip and a primary button are in the header's example of every spatial stylus, but an accessory that
/// reports neither is one where reading the profile is the answer rather than assuming them.
fn press(
    input: &ProtocolObject<dyn GCDevicePhysicalInput>,
    name: &GCInputButtonName,
) -> Option<Press> {
    // SAFETY: the input profile is live, and the collection and element it answers with are the framework's
    // own objects, dropped at the end of the call.
    unsafe {
        let button = input.buttons().elementForAlias(name)?;
        // The header types a button's pressed input as conforming to both `GCPressedStateInput` and
        // `GCLinearInput`, which the bindings cannot express as one type - so the casts say what the header
        // says rather than what the generated type could.
        let pressed = button.pressedInput();
        let state =
            Retained::cast_unchecked::<ProtocolObject<dyn GCPressedStateInput>>(pressed.clone());
        let value = Retained::cast_unchecked::<ProtocolObject<dyn GCLinearInput>>(pressed);
        Some(Press {
            pressed: state.isPressed(),
            value: value.value(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tip's contact is the selection and its force is the trigger; the front button is the squeeze.
    #[test]
    fn a_tips_press_is_a_selection_and_its_force_is_the_trigger() {
        let (buttons, axes) = mapped(
            Some(Press {
                pressed: true,
                value: 0.5,
            }),
            true,
        );
        assert!(buttons.select);
        assert!(buttons.squeeze);
        assert!(!buttons.menu, "the secondary button is not the menu button");
        assert_eq!(axes.trigger, 0.5);
        assert_eq!(axes.thumbstick, wxr::glam::Vec2::ZERO);
    }

    /// A tip the profile does not have is a tip at rest, not a pressed one at zero force.
    #[test]
    fn a_stylus_without_a_tip_reports_no_selection() {
        let (buttons, axes) = mapped(None, false);
        assert_eq!(buttons, wxr::Buttons::default());
        assert_eq!(axes, wxr::Axes::default());
    }

    /// Force alone is not a press: a tip can rest on a surface with nothing behind it, and the button is
    /// what the framework says it is.
    #[test]
    fn a_tip_that_is_not_pressed_is_not_a_selection() {
        let (buttons, axes) = mapped(
            Some(Press {
                pressed: false,
                value: 0.25,
            }),
            false,
        );
        assert!(!buttons.select);
        assert_eq!(axes.trigger, 0.25);
    }
}
