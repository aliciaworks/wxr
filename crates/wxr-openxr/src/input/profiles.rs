//! What each interaction profile offers, and what a suggestion for it has to be made of.
//!
//! `xrSuggestInteractionProfileBindings` is all-or-nothing: one binding to a path the profile does not have
//! is the whole profile refused, and a refused profile binds nothing at all. That is why this is a table and
//! not one list of paths applied to every profile - a simple controller has no trigger, an Oculus Touch has
//! no click on its trigger, and suggesting either costs the profile every binding it would have had.
//!
//! The profiles named here are the ones Khronos' own `hello_xr` suggests, plus the Vive controller: the set
//! that the runtimes behind most headsets recognise. A controller outside the table is not a controller with
//! no input - the runtime falls back to the simple controller, which is why that one is first and why every
//! runtime has to understand it.

use openxr as xr;

use super::{Hand, Hands};

/// Which of the set's actions a subpath drives.
///
/// The action types are the reason this is an enum and not the binding itself: a binding is typed by the
/// action it was made for, so a list holding them would have to be one of the four types. A control names
/// what the action is about, and [`Hand::bind`] is the one place that knows its type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Control {
    /// Where the hand is held.
    Grip,
    /// Where it points.
    Aim,
    /// The main button.
    Select,
    /// The grip button.
    Squeeze,
    /// A menu or system button.
    Menu,
    /// The analogue trigger, the same control `Select` is on most controllers.
    Trigger,
    /// The analogue grip, the same control `Squeeze` is on some controllers.
    SqueezeValue,
    /// The stick.
    Thumbstick,
    /// The actuator.
    Haptic,
}

/// Which hands a component exists on.
///
/// Most are on both, but Oculus puts the menu button on the left controller only - its right one has a system
/// button, which is the runtime's - and a path naming the right hand's menu is not a component of that
/// profile. One such path takes the whole profile down with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sides {
    Both,
    Left,
}

impl Sides {
    fn has(self, handedness: wxr::Handedness) -> bool {
        match self {
            Sides::Both => true,
            Sides::Left => handedness == wxr::Handedness::Left,
        }
    }
}

/// One binding: what is bound, where, and on which hands.
type Entry = (Control, &'static str, Sides);

/// A profile, and every part of this set that it can be bound to.
struct Profile {
    /// The runtime's path for it, which is what the suggestion is made with.
    path: &'static str,
    entries: &'static [Entry],
}

/// The profiles worth suggesting, and what each of them has.
///
/// A grip pose, an aim pose and a haptic are in every one of them. The buttons, the stick and the analogue
/// trigger are what differ, and the differences are what this table is for.
const PROFILES: &[Profile] = &[
    // The one every runtime must know, and the one it falls back to for a controller nothing here names. It
    // has a select and a menu and no trigger, no stick and no grip button.
    Profile {
        path: "/interaction_profiles/khr/simple_controller",
        entries: &[
            (Control::Grip, "/input/grip/pose", Sides::Both),
            (Control::Aim, "/input/aim/pose", Sides::Both),
            (Control::Select, "/input/select/click", Sides::Both),
            (Control::Menu, "/input/menu/click", Sides::Both),
            (Control::Haptic, "/output/haptic", Sides::Both),
        ],
    },
    // HTC's wand: a trigger that is also the select, a grip button of its own, and a trackpad where the
    // others have a stick - which is why a Vive has no thumbstick to bind.
    Profile {
        path: "/interaction_profiles/htc/vive_controller",
        entries: &[
            (Control::Grip, "/input/grip/pose", Sides::Both),
            (Control::Aim, "/input/aim/pose", Sides::Both),
            (Control::Select, "/input/trigger/click", Sides::Both),
            (Control::Squeeze, "/input/squeeze/click", Sides::Both),
            (Control::Menu, "/input/menu/click", Sides::Both),
            (Control::Trigger, "/input/trigger/value", Sides::Both),
            (Control::Haptic, "/output/haptic", Sides::Both),
        ],
    },
    // Windows Mixed Reality: a grip button, a trigger with no click of its own, and both a stick and a pad.
    Profile {
        path: "/interaction_profiles/microsoft/motion_controller",
        entries: &[
            (Control::Grip, "/input/grip/pose", Sides::Both),
            (Control::Aim, "/input/aim/pose", Sides::Both),
            (Control::Squeeze, "/input/squeeze/click", Sides::Both),
            (Control::Menu, "/input/menu/click", Sides::Both),
            (Control::Trigger, "/input/trigger/value", Sides::Both),
            (Control::Thumbstick, "/input/thumbstick", Sides::Both),
            (Control::Haptic, "/output/haptic", Sides::Both),
        ],
    },
    // The Touch controller: a trigger that is the select, a grip that is an analogue squeeze, and a menu on
    // the left hand only.
    Profile {
        path: "/interaction_profiles/oculus/touch_controller",
        entries: &[
            (Control::Grip, "/input/grip/pose", Sides::Both),
            (Control::Aim, "/input/aim/pose", Sides::Both),
            (Control::Menu, "/input/menu/click", Sides::Left),
            (Control::Trigger, "/input/trigger/value", Sides::Both),
            (Control::SqueezeValue, "/input/squeeze/value", Sides::Both),
            (Control::Thumbstick, "/input/thumbstick", Sides::Both),
            (Control::Haptic, "/output/haptic", Sides::Both),
        ],
    },
    // Valve's knuckles: the trigger has both a click and a value, the grip is analogue only, and the system
    // button is the runtime's rather than a game's.
    Profile {
        path: "/interaction_profiles/valve/index_controller",
        entries: &[
            (Control::Grip, "/input/grip/pose", Sides::Both),
            (Control::Aim, "/input/aim/pose", Sides::Both),
            (Control::Select, "/input/trigger/click", Sides::Both),
            (Control::Trigger, "/input/trigger/value", Sides::Both),
            (Control::SqueezeValue, "/input/squeeze/value", Sides::Both),
            (Control::Thumbstick, "/input/thumbstick", Sides::Both),
            (Control::Haptic, "/output/haptic", Sides::Both),
        ],
    },
];

/// Tell the runtime what each action would mean on each profile.
///
/// A failure is logged rather than discarded. The call binds a whole profile or none of it, so a rejection
/// leaves every button on that controller dead - which shows up as "the trigger does nothing on this
/// headset" and gives nobody anything to go on.
pub(super) fn suggest(instance: &xr::Instance, hands: &Hands) {
    for profile in PROFILES {
        let Ok(path) = instance.string_to_path(profile.path) else {
            continue;
        };
        let mut bindings = Vec::new();
        for hand in &hands.hands {
            let side = match hand.handedness {
                wxr::Handedness::Left => "left",
                _ => "right",
            };
            for &(control, subpath, sides) in profile.entries {
                if sides.has(hand.handedness) {
                    hand.bind(instance, side, control, subpath, &mut bindings);
                }
            }
        }
        if let Err(error) = instance.suggest_interaction_profile_bindings(path, &bindings) {
            log::warn!(
                "wxr-openxr: the runtime refused {}: {error:?}",
                profile.path
            );
        }
    }
}

impl Hand {
    /// Add this control on this hand, if the runtime knows the path.
    ///
    /// A path the runtime does not know is skipped rather than suggested: the suggestion would fail for the
    /// whole profile, and the profile is worth more than the one binding.
    fn bind<'a>(
        &'a self,
        instance: &xr::Instance,
        side: &str,
        control: Control,
        subpath: &str,
        bindings: &mut Vec<xr::Binding<'a>>,
    ) {
        let full = format!("/user/hand/{side}{subpath}");
        let Ok(path) = instance.string_to_path(&full) else {
            return;
        };
        bindings.push(match control {
            Control::Grip => xr::Binding::new(&self.grip_action, path),
            Control::Aim => xr::Binding::new(&self.aim_action, path),
            Control::Select => xr::Binding::new(&self.select, path),
            Control::Squeeze => xr::Binding::new(&self.squeeze, path),
            Control::SqueezeValue => xr::Binding::new(&self.squeeze_value, path),
            Control::Menu => xr::Binding::new(&self.menu, path),
            Control::Trigger => xr::Binding::new(&self.trigger, path),
            Control::Thumbstick => xr::Binding::new(&self.thumbstick, path),
            Control::Haptic => match &self.haptic {
                Some(haptic) => xr::Binding::new(haptic, path),
                // The action was never made because the runtime has no haptics, so there is nothing to
                // bind - and a suggestion naming an action the instance does not have would be refused.
                None => return,
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path is the specification's, and the plural is the part that matters: a runtime knows
    /// `/interaction_profiles/...` and has never heard of a `/interaction_profile/...`.
    #[test]
    fn every_profile_is_named_as_the_specification_names_it() {
        for profile in PROFILES {
            assert!(
                profile.path.starts_with("/interaction_profiles/"),
                "{}",
                profile.path
            );
        }
    }

    /// Every subpath is an input or an output, because that is all a hand has.
    #[test]
    fn every_binding_is_an_input_or_an_output() {
        for profile in PROFILES {
            for &(_, subpath, _) in profile.entries {
                assert!(
                    subpath.starts_with("/input/") || subpath.starts_with("/output/"),
                    "{}: {subpath}",
                    profile.path
                );
            }
        }
    }

    /// A controller with no grip pose or no aim pose would be a controller that cannot be pointed at
    /// anything, and every profile here has both.
    #[test]
    fn every_profile_has_a_grip_and_an_aim() {
        for profile in PROFILES {
            for control in [Control::Grip, Control::Aim] {
                assert!(
                    profile.entries.iter().any(|entry| entry.0 == control),
                    "{} has no {control:?}",
                    profile.path
                );
            }
        }
    }
}
