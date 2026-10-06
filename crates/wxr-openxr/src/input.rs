//! OpenXR input: the actions, the bindings that make them mean something, and the spaces they resolve to.
//!
//! OpenXR does not have controllers; it has **actions** and **interaction profiles**. An action is what the
//! game wants - "where the left hand is" - and a profile is what a particular controller happens to offer.
//! The two are joined by *suggested bindings*, which is the runtime being told "if this is a Touch
//! controller, the grip pose is that button", and the runtime choosing among the profiles the user has.
//!
//! That indirection is the whole point, and the reason this is a hundred lines instead of ten: a game that
//! read a controller's buttons directly would work on the controller it was written against and nowhere
//! else. It is also why the bindings below are *suggestions* - a runtime is free to disagree, and a game
//! that insisted would be a game that does not run on somebody's headset.
//!
//! Poses are the half a core can express today, and they are read the way OpenXR insists on: a pose action
//! becomes a *space*, and the space is located against the reference space the caller asked in.
//! `locate_space` is the call WebXR's `getPose` is, which is not a coincidence - the two specifications are
//! describing the same hardware.

use openxr as xr;

use crate::Error;

/// The hands, and where they are.
pub struct Hands {
    /// One action set, because one game. A set is a group of actions a runtime enables and disables
    /// together, and splitting a game's inputs into several would be inventing a decision nobody is making.
    set: xr::ActionSet,
    hands: Vec<Hand>,
}

struct Hand {
    handedness: wxr::Handedness,
    /// `/user/hand/left` or `/user/hand/right`: which physical hand the actions are read for.
    path: xr::Path,
    grip_action: xr::Action<xr::Posef>,
    aim_action: xr::Action<xr::Posef>,
    grip: xr::Space,
    aim: xr::Space,
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
            let grip_action = set
                .create_action::<xr::Posef>(&format!("{side}_grip"), "Grip", &[path])
                .map_err(|error| Error::runtime("create a pose action", error))?;
            let aim_action = set
                .create_action::<xr::Posef>(&format!("{side}_aim"), "Aim", &[path])
                .map_err(|error| Error::runtime("create a pose action", error))?;

            // A space per action, because that is how a pose action is read: located against whatever
            // reference space the caller asked in.
            let grip = grip_action
                .create_space(session, path, xr::Posef::IDENTITY)
                .map_err(|error| Error::runtime("make the grip space", error))?;
            let aim = aim_action
                .create_space(session, path, xr::Posef::IDENTITY)
                .map_err(|error| Error::runtime("make the aim space", error))?;
            hands.push(Hand {
                handedness,
                path,
                grip_action,
                aim_action,
                grip,
                aim,
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
    /// rather than refusing - and it is the other inputs, triggers and buttons, that a missing profile
    /// costs. A suggestion the runtime does not recognise is ignored by it, so failing here is not failing.
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
                if let Ok(path) =
                    instance.string_to_path(&format!("/user/hand/{side}/input/grip/pose"))
                {
                    bindings.push(xr::Binding::new(&hand.grip_action, path));
                }
                if let Ok(path) =
                    instance.string_to_path(&format!("/user/hand/{side}/input/aim/pose"))
                {
                    bindings.push(xr::Binding::new(&hand.aim_action, path));
                }
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

    /// Where the hands are, in the space given.
    pub fn read(&self, base: &xr::Space, time: xr::Time, out: &mut Vec<wxr::InputSource>) {
        for hand in &self.hands {
            for (grip, space) in [(wxr::Grip::Grip, &hand.grip), (wxr::Grip::Aim, &hand.aim)] {
                let Ok(location) = space.locate(base, time) else {
                    continue;
                };
                // Tracked is a question about the position *and* the orientation, and a runtime can know one
                // without the other - a hand behind the user's back has a direction and no place.
                let tracked = location
                    .location_flags
                    .contains(xr::SpaceLocationFlags::POSITION_TRACKED)
                    && location
                        .location_flags
                        .contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED);
                out.push(wxr::InputSource {
                    handedness: hand.handedness,
                    grip,
                    pose: crate::pose(location.pose),
                    tracked,
                });
            }
            // The path is what the actions were created for and is kept for the buttons that will be added
            // to them; a pose does not need it once its space exists.
            let _ = hand.path;
        }
    }
}
