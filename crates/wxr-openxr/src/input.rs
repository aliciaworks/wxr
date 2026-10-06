//! OpenXR input: the actions, the bindings that make them mean something, and the spaces they resolve to.
//!
//! OpenXR does not have controllers; it has **actions** and **interaction profiles**. An action is what the
//! game wants - "where the left hand is", "is the trigger pulled" - and a profile is what a particular
//! controller happens to offer. The two are joined by *suggested bindings*, which is the runtime being told
//! "if this is a Touch controller, the grip pose is that button", and the runtime choosing among the
//! profiles the user has.
//!
//! That indirection is the whole point, and the reason this is not ten lines: a game that read a
//! controller's buttons directly would work on the controller it was written against and nowhere else. It
//! is also why the bindings below are *suggestions* - a runtime is free to disagree, and a game that
//! insisted would be a game that does not run on somebody's headset.
//!
//! Poses are read the way OpenXR insists on, and the way WebXR does it too: a pose action becomes a space,
//! and the space is located against the reference space the caller asked in. `locate_space` and `getPose`
//! are the same call in two specifications, which is not a coincidence - they describe the same hardware.

use openxr as xr;

use crate::Error;

/// The hands, and what they are doing.
pub struct Hands {
    /// One action set, because one game. A set is a group of actions a runtime enables and disables
    /// together, and splitting a game's inputs into several would be inventing a decision nobody is making.
    set: xr::ActionSet,
    hands: Vec<Hand>,
}

struct Hand {
    handedness: wxr::Handedness,
    /// `/user/hand/left` or `/user/hand/right`: which physical hand everything below is read for.
    path: xr::Path,
    /// The pose actions are kept as well as the spaces made from them: a space is created from an action,
    /// and the action is destroyed when it is dropped.
    grip_action: xr::Action<xr::Posef>,
    aim_action: xr::Action<xr::Posef>,
    grip: xr::Space,
    aim: xr::Space,
    select: xr::Action<bool>,
    squeeze: xr::Action<bool>,
    menu: xr::Action<bool>,
    trigger: xr::Action<f32>,
    thumbstick: xr::Action<xr::Vector2f>,
}

impl Hands {
    /// Declare the actions, suggest what they mean, and attach them to the session.
    pub fn new(instance: &xr::Instance, session: &xr::Session<xr::Vulkan>) -> Result<Self, Error> {
        let set = instance
            .create_action_set("hands", "Hands", 0)
            .map_err(|error| Error::runtime("create the action set", error))?;

        let mut hands = Vec::new();
        for (handedness, side) in [
            (wxr::Handedness::Left, "left"),
            (wxr::Handedness::Right, "right"),
        ] {
            let path = instance
                .string_to_path(&format!("/user/hand/{side}"))
                .map_err(|error| Error::runtime("name a hand", error))?;
            // OpenXR requires every action's *localized* name in a set to be distinct, and a pair of hands
            // declaring a "Grip" each is two the same - so the side belongs in the name a person reads as
            // well as in the one the code uses.
            let pose = |name: &str, localized: &str| {
                set.create_action::<xr::Posef>(
                    &format!("{side}_{name}"),
                    &format!("{side} {localized}"),
                    &[path],
                )
            };
            let grip_action = pose("grip", "Grip")
                .map_err(|error| Error::runtime("create a pose action", error))?;
            let aim_action = pose("aim", "Aim")
                .map_err(|error| Error::runtime("create a pose action", error))?;

            // A space per pose action, because that is how a pose is read: located against whatever
            // reference space the caller asked in.
            let grip = grip_action
                .create_space(session, path, xr::Posef::IDENTITY)
                .map_err(|error| Error::runtime("make the grip space", error))?;
            let aim = aim_action
                .create_space(session, path, xr::Posef::IDENTITY)
                .map_err(|error| Error::runtime("make the aim space", error))?;

            let boolean = |name: &str, localized: &str| {
                set.create_action::<bool>(
                    &format!("{side}_{name}"),
                    &format!("{side} {localized}"),
                    &[path],
                )
            };
            hands.push(Hand {
                handedness,
                path,
                grip_action,
                aim_action,
                grip,
                aim,
                select: boolean("select", "Select")
                    .map_err(|error| Error::runtime("create an action", error))?,
                squeeze: boolean("squeeze", "Squeeze")
                    .map_err(|error| Error::runtime("create an action", error))?,
                menu: boolean("menu", "Menu")
                    .map_err(|error| Error::runtime("create an action", error))?,
                trigger: set
                    .create_action::<f32>(
                        &format!("{side}_trigger"),
                        &format!("{side} trigger"),
                        &[path],
                    )
                    .map_err(|error| Error::runtime("create an action", error))?,
                thumbstick: set
                    .create_action::<xr::Vector2f>(
                        &format!("{side}_thumbstick"),
                        &format!("{side} thumbstick"),
                        &[path],
                    )
                    .map_err(|error| Error::runtime("create an action", error))?,
            });
        }

        let hands = Self { set, hands };
        hands.suggest(instance);
        session
            .attach_action_sets(&[&hands.set])
            .map_err(|error| Error::runtime("attach the action set", error))?;
        Ok(hands)
    }

    /// Tell the runtime what each action would mean on each of the profiles worth naming.
    ///
    /// The simple controller every runtime must understand, and the three that cover most of what people
    /// own. A controller outside the list still gets its poses - a runtime falls back to what it knows
    /// rather than refusing - and it is the buttons that a missing profile costs. A suggestion a runtime
    /// does not recognise is ignored by it, so failing here is not failing.
    fn suggest(&self, instance: &xr::Instance) {
        for profile in [
            "/interaction_profile/khr/simple_controller",
            "/interaction_profile/oculus/touch_controller",
            "/interaction_profile/valve/index_controller",
            "/interaction_profile/microsoft/mixed_reality_controller",
        ] {
            let Ok(profile) = instance.string_to_path(profile) else {
                continue;
            };
            let mut bindings = Vec::new();
            for hand in &self.hands {
                let side = match hand.handedness {
                    wxr::Handedness::Left => "left",
                    _ => "right",
                };
                bind(
                    instance,
                    side,
                    &hand.grip_action,
                    "/input/grip/pose",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.aim_action,
                    "/input/aim/pose",
                    &mut bindings,
                );
                // The names here are the specification's, not this crate's: `select`, `squeeze` and
                // `thumbstick` are the common ones, and a profile with a trigger of its own says so under
                // its own name - which is why the trigger is suggested twice.
                bind(
                    instance,
                    side,
                    &hand.select,
                    "/input/select/click",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.squeeze,
                    "/input/squeeze/click",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.menu,
                    "/input/menu/click",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.trigger,
                    "/input/trigger/value",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.trigger,
                    "/input/select/value",
                    &mut bindings,
                );
                bind(
                    instance,
                    side,
                    &hand.thumbstick,
                    "/input/thumbstick",
                    &mut bindings,
                );
            }
            let _ = instance.suggest_interaction_profile_bindings(profile, &bindings);
        }
    }

    /// Ask the runtime for this frame's action states. Nothing can be read before this, and it is once per
    /// frame rather than once per action.
    pub fn sync(&self, session: &xr::Session<xr::Vulkan>) -> Result<(), Error> {
        session
            .sync_actions(&[xr::ActiveActionSet::new(&self.set)])
            .map_err(|error| Error::runtime("sync the actions", error))
    }

    /// Where the hands are and what they are doing, in the space given.
    pub fn read(
        &self,
        session: &xr::Session<xr::Vulkan>,
        base: &xr::Space,
        time: xr::Time,
        out: &mut Vec<wxr::InputSource>,
    ) {
        for hand in &self.hands {
            let grip = hand.grip.locate(base, time).ok();
            let aim = hand.aim.locate(base, time).ok();
            let pose = |location: &Option<xr::SpaceLocation>| {
                location
                    .as_ref()
                    .map(|location| crate::pose(location.pose))
                    .unwrap_or(wxr::Pose::IDENTITY)
            };
            let tracked = |location: &Option<xr::SpaceLocation>| {
                location.as_ref().is_some_and(|location| {
                    // Tracked is a question about the position *and* the orientation: a hand behind the
                    // user's back has a direction and no place.
                    location
                        .location_flags
                        .contains(xr::SpaceLocationFlags::POSITION_TRACKED)
                        && location
                            .location_flags
                            .contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED)
                })
            };

            let boolean = |action: &xr::Action<bool>| {
                action
                    .state::<_>(session, hand.path)
                    .map(|state| state.current_state)
                    .unwrap_or(false)
            };
            out.push(wxr::InputSource {
                handedness: hand.handedness,
                grip: pose(&grip),
                aim: pose(&aim),
                tracked: tracked(&grip) || tracked(&aim),
                buttons: wxr::Buttons {
                    select: boolean(&hand.select),
                    squeeze: boolean(&hand.squeeze),
                    menu: boolean(&hand.menu),
                },
                axes: wxr::Axes {
                    trigger: hand
                        .trigger
                        .state::<xr::Vulkan>(session, hand.path)
                        .map(|state| state.current_state)
                        .unwrap_or(0.0),
                    thumbstick: hand
                        .thumbstick
                        .state::<xr::Vulkan>(session, hand.path)
                        .map(|state| {
                            wxr::glam::Vec2::new(state.current_state.x, state.current_state.y)
                        })
                        .unwrap_or_default(),
                },
            });
        }
    }
}

/// Suggest one action on one path, if the runtime knows the path.
///
/// A `fn` and not a closure because the actions are typed: the same binding is suggested for a `bool`, an
/// `f32`, a `Vector2f` and a pose, and a closure would be one of those four.
fn bind<'a, T: xr::ActionTy>(
    instance: &xr::Instance,
    side: &str,
    action: &'a xr::Action<T>,
    path: &str,
    bindings: &mut Vec<xr::Binding<'a>>,
) {
    if let Ok(path) = instance.string_to_path(&format!("/user/hand/{side}{path}")) {
        bindings.push(xr::Binding::new(action, path));
    }
}
