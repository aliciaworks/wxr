//! The input sources and the hand.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn inputs_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        let Some(session) = self.inner.borrow().session.clone() else {
            return Ok(());
        };

        // A gamepad is the `xr-standard` mapping, which is a specification of its own and is what makes
        // the button order below mean anything: the trigger first, the squeeze second, the stick's click
        // fourth, and the stick's two axes after the touchpad's. A profile that does not follow it is a
        // profile this reads the wrong way - which is a thing the gamepad's own `mapping` says, and a thing
        // to handle the day one shows up.
        let held = session.input_sources();
        for index in 0..held.length() {
            let Some(source) = held.get(index) else {
                continue;
            };
            let handedness = match source.handedness() {
                XrHandedness::Left => wxr::Handedness::Left,
                XrHandedness::Right => wxr::Handedness::Right,
                XrHandedness::None | XrHandedness::__Invalid => wxr::Handedness::Unknown,
            };

            let grip = source
                .grip_space()
                .and_then(|space| frame.get_pose(&space, &reference));
            let aim = frame.get_pose(&source.target_ray_space(), &reference);

            // The generated attribute names the DOM type and this backend leaves it to `web-sys`: a gamepad
            // is not WebXR's object, and mirroring it would be a second Rust type for one thing.
            let gamepad = (!source.gamepad().is_null_or_undefined())
                .then(|| source.gamepad())
                .and_then(|value| value.dyn_into::<web_sys::Gamepad>().ok());
            let button = |index: u32| {
                gamepad.as_ref().and_then(|gamepad| {
                    gamepad
                        .buttons()
                        .get(index)
                        .dyn_into::<web_sys::GamepadButton>()
                        .ok()
                })
            };
            let axis = |index: u32| {
                gamepad
                    .as_ref()
                    .and_then(|gamepad| gamepad.axes().get(index).as_f64())
                    .unwrap_or(0.0) as f32
            };

            // A source with no grip pose is an aim and nothing to hold - a gaze cursor - and it is not
            // untracked: it has a direction, and the direction is the whole of it.
            let tracked = grip.is_some() || aim.is_some();
            out.push(wxr::InputSource {
                // The same map the event handlers use, so a frame and an event name the same source the same
                // way - which is the whole reason the core has an id where WebXR has an object.
                id: self.sources.id(&source),
                handedness,
                target_ray_mode: input::target_ray_mode(&source),
                // A source with a `hand` is one with a skeleton to ask for, which is the whole of what WebXR
                // says about it: whether the fingers are tracked is `hand`'s answer, not this one's.
                hand: source.hand().is_some(),
                // And whether there is an actuator to send to, which is the gamepad's list being non-empty -
                // the one thing about a source that WebXR answers through the gamepad rather than through the
                // source itself.
                haptics: haptics::actuator(&source).is_some(),
                grip: grip
                    .map(|pose| transform(pose.transform()))
                    .unwrap_or(wxr::Pose::IDENTITY),
                aim: aim
                    .map(|pose| transform(pose.transform()))
                    .unwrap_or(wxr::Pose::IDENTITY),
                tracked,
                buttons: wxr::Buttons {
                    select: button(0).is_some_and(|button| button.pressed()),
                    squeeze: button(1).is_some_and(|button| button.pressed()),
                    menu: button(3).is_some_and(|button| button.pressed()),
                },
                axes: wxr::Axes {
                    trigger: button(0).map(|button| button.value() as f32).unwrap_or(0.0),
                    thumbstick: wxr::glam::Vec2::new(axis(2), axis(3)),
                },
            });
        }
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn hand_impl(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(input) = self.sources.get(source) else {
            return Ok(());
        };
        let Some(hand) = input.hand() else {
            return Ok(());
        };
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        // A joint at a time, because a joint is the call the browser has - and a joint it will not answer for is
        // a joint that is not tracked, which is what `None` in an empty `Hand` already means.
        for joint in wxr::HandJoint::ALL {
            let space = hand.get(hand_joint(joint));
            if let Some(pose) = frame.get_joint_pose(&space, &reference) {
                out.joints_mut()[joint.index()] = Some(wxr::Joint {
                    pose: transform(pose.transform()),
                    radius: pose.radius(),
                });
            }
        }
        Ok(())
    }
}
