//! The eyes, which are the one source that is not a hand.
//!
//! `XR_EXT_eye_gaze_interaction` is a profile like the others and a user path like no other: `/user/eyes_ext`
//! rather than a hand, and one pose - where the wearer is looking. The core has the shape for that already:
//! [`wxr::TargetRayMode::Gaze`] and a [`wxr::Handedness::Unknown`] source, which is what WebXR reports for the
//! same thing on a runtime that has it.
//!
//! There is no select and no squeeze here, and that is the specification rather than this file: a gaze ray is
//! a pointer for choosing with, and the choosing is a hand's button or a dwell the app keeps time for. So the
//! source carries a pose and the default buttons, and it makes no events of its own.
//!
//! An instance can have the extension and the session no permission for eye tracking - the data-protection
//! kind rather than this crate's - and then the pose locates untracked and the source is still reported. An
//! app that draws a cursor on it is an app that can say the eyes are there and are not being followed.

use super::*;

/// The profile the pose is suggested under.
const PROFILE: &str = "/interaction_profiles/ext/eye_gaze_interaction";
/// The user path and the component, which are one string each because the suggestion is made with the pair.
const USER: &str = "/user/eyes_ext";
const POSE: &str = "/input/gaze_ext/pose";

/// The number the gaze source is named by.
///
/// The two hands are 0 and 1, and this is the next one. A source's id is only meaningful to the backend that
/// handed it out, and it has to stay the same for as long as the source exists - which for the eyes is as
/// long as the session.
const ID: u32 = 2;

/// Whether this system has eyes to track, which the extension alone cannot say.
///
/// `XR_EXT_eye_gaze_interaction` is an *instance* extension and the eyes are the *system's*: a runtime built
/// for headsets with eye tracking can be running one without, and the property the profile's own chapter
/// starts with is where it says which. Reporting a source without asking is how every app that lists the
/// inputs gets a gaze ray that can never be tracked.
pub(crate) fn supported(instance: &xr::Instance, system: xr::SystemId) -> bool {
    let mut gaze = openxr_sys::SystemEyeGazeInteractionPropertiesEXT {
        ty: openxr_sys::SystemEyeGazeInteractionPropertiesEXT::TYPE,
        next: std::ptr::null_mut(),
        supports_eye_gaze_interaction: openxr_sys::FALSE,
    };
    // SAFETY: zero is a valid initial value for every remaining field - a system name of empty string, a
    // vendor of zero and two properties of false - and the `next` pointer below points at `gaze`, which
    // outlives the call. The runtime fills both structures and reads neither of the zeroed fields.
    let mut properties = openxr_sys::SystemProperties {
        ty: openxr_sys::SystemProperties::TYPE,
        next: std::ptr::from_mut(&mut gaze).cast(),
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: the instance is live and the function pointer came from it, and both structures are the ones
    // the call names in their `ty` with the chain the runtime walks.
    let result = unsafe {
        (instance.fp().get_system_properties)(instance.as_raw(), system, &mut properties)
    };
    result == openxr_sys::Result::SUCCESS && bool::from(gaze.supports_eye_gaze_interaction)
}

/// The gaze action and the space it is read through.
pub(super) struct Gaze {
    /// The action is kept although only the space is located: a space is made from an action, and a binding
    /// names the action rather than the space.
    action: xr::Action<xr::Posef>,
    space: xr::Space,
}

impl Gaze {
    /// Make the action and the space, on the eyes' own user path.
    pub(super) fn new(
        instance: &xr::Instance,
        session: &xr::Session<xr::Vulkan>,
        set: &xr::ActionSet,
    ) -> Result<Self, Error> {
        let path = instance
            .string_to_path(USER)
            .map_err(|error| Error::runtime("name the eyes", error))?;
        // The subaction path is the user path here and not a `/user/hand/...`: there is one pair of eyes, so
        // there is nothing for a subaction to choose between.
        let action = set
            .create_action::<xr::Posef>("gaze", "Gaze", &[path])
            .map_err(|error| Error::runtime("create the gaze action", error))?;
        let space = action
            .create_space(session, path, xr::Posef::IDENTITY)
            .map_err(|error| Error::runtime("make the gaze space", error))?;
        Ok(Self { action, space })
    }

    /// Tell the runtime what the action means, which is the one component the profile has.
    pub(super) fn suggest(&self, instance: &xr::Instance) {
        let (Ok(profile), Ok(binding)) = (
            instance.string_to_path(PROFILE),
            instance.string_to_path(&format!("{USER}{POSE}")),
        ) else {
            return;
        };
        let bindings = [xr::Binding::new(&self.action, binding)];
        if let Err(error) = instance.suggest_interaction_profile_bindings(profile, &bindings) {
            log::warn!("wxr-openxr: the runtime refused the eye gaze profile: {error:?}");
        }
    }

    /// The profile name for the eyes' source, and `None` for any other.
    ///
    /// The eyes have one profile and the runtime does not report it per source, unlike a hand's - so this is
    /// the fixed half of what `Session::profiles` answers.
    pub(super) fn profile_of(&self, source: wxr::InputId) -> Option<&'static str> {
        (source == wxr::InputId::new(ID)).then_some(PROFILE)
    }

    /// This frame's source, which is one pose and nothing else.
    pub(super) fn push(&self, base: &xr::Space, time: xr::Time, out: &mut Vec<wxr::InputSource>) {
        // The velocity comes back beside the location from the same call, and a gaze ray is a thing moving
        // with the head: an app that fades a cursor by speed has as much use for it as for the pose.
        let located = self.space.relate(base, time).ok();
        out.push(wxr::InputSource {
            id: wxr::InputId::new(ID),
            // Not a hand: the eyes belong to the head, and the core has a name for a source that is not
            // one.
            handedness: wxr::Handedness::Unknown,
            target_ray_mode: wxr::TargetRayMode::Gaze,
            hand: false,
            haptics: false,
            // Nothing here has a model for the runtime to draw: a gaze ray is not shown at all.
            skip_rendering: false,
            // A gaze ray has no place the source is held - and the core's grip is a pose rather than an
            // option, so it is the identity, the same answer `read` gives a hand whose grip was not
            // located. What an app is meant to use is `aim`.
            grip: wxr::Pose::IDENTITY,
            aim: located
                .as_ref()
                .map(|(location, _)| crate::pose(location.pose))
                .unwrap_or(wxr::Pose::IDENTITY),
            motion: located
                .as_ref()
                .and_then(|(_, velocity)| crate::motion(velocity)),
            tracked: located
                .as_ref()
                .is_some_and(|(location, _)| tracked_one(location)),
            buttons: wxr::Buttons::default(),
            axes: wxr::Axes::default(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The suggestion is made with the full path, and the user path is not a hand's: a gaze binding under
    /// `/user/hand/...` would take the whole profile down with it.
    #[test]
    fn the_binding_is_on_the_eyes_own_path() {
        assert_eq!(
            format!("{USER}{POSE}"),
            "/user/eyes_ext/input/gaze_ext/pose"
        );
        assert!(PROFILE.starts_with("/interaction_profiles/"));
    }

    /// The eyes are a source of their own: their number is not one of the two hands'.
    #[test]
    fn the_eyes_are_not_a_hand() {
        assert_ne!(ID, 0);
        assert_ne!(ID, 1);
    }
}
