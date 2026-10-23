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
        // What is held now, not what was held last frame: the caller's vector is the one a frame loop
        // keeps, and every other list the session fills starts with the same line.
        out.clear();
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some(frame) = self.frame() else {
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
                    // No `xr-standard` button is a menu: its fourth is the thumbstick's click, which is a
                    // game's to bind rather than the system button this field is for. A profile with a
                    // menu of its own has nowhere in the mapping to put it, and saying otherwise would
                    // report a stick press as something the platform may not let a game have.
                    menu: false,
                },
                axes: wxr::Axes {
                    trigger: button(0).map(|button| button.value() as f32).unwrap_or(0.0),
                    // A gamepad's Y axis is positive down - the Gamepad specification calls -1
                    // "forward" - and the core's is positive up.
                    thumbstick: wxr::glam::Vec2::new(axis(2), -axis(3)),
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
        let Some(frame) = self.frame() else {
            return Ok(());
        };
        // The bulk way first, on a browser that has it: `fillPoses` and `fillJointRadii` are the
        // specification's answer to a skeleton being fifty questions, and a frame that answers them both has
        // answered for all twenty-five joints in two calls.
        if self.hand_in_bulk(&frame, &hand, &reference, out) {
            return Ok(());
        }
        // The way every browser can be asked, a joint at a time - and the same answer over more calls. A joint
        // it will not answer for is a joint that is not tracked, which is what `None` in an empty `Hand` already
        // means.
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

    /// Every joint of one hand in two calls, and whether the browser answered for all of them.
    ///
    /// Both or neither, and never a mixture: a [`wxr::Joint`] is a pose *and* a radius, and one of the two
    /// half-answered is a joint the caller would have to guess the rest of. They also fail together - both
    /// answer `false` when any one space had no pose - so there is nothing to gain from taking one without the
    /// other.
    ///
    /// `false` is not a failure, it is the common case: a browser without these methods, or a hand half out of
    /// view. A hand with one joint untracked is a hand whose joints want telling apart, which is what the
    /// joint-at-a-time path above can do and a bulk answer cannot say.
    fn hand_in_bulk(
        &self,
        frame: &XrFrame,
        hand: &XrHand,
        reference: &XrSpace,
        out: &mut wxr::Hand,
    ) -> bool {
        let spaces = js_sys::Array::new_with_length(wxr::HandJoint::COUNT as u32);
        for (index, joint) in wxr::HandJoint::ALL.iter().enumerate() {
            spaces.set(index as u32, JsValue::from(hand.get(hand_joint(*joint))));
        }
        let matrices = js_sys::Float32Array::new_with_length((wxr::HandJoint::COUNT * 16) as u32);
        let radii = js_sys::Float32Array::new_with_length(wxr::HandJoint::COUNT as u32);
        // Both caught, because a method that is not there is the ordinary answer rather than the broken one -
        // `fillPoses` is in the same specification module as the hand, but not in every browser that has hands.
        let posed = frame
            .fill_poses_with_f32_array(&spaces, reference, &matrices)
            .unwrap_or(false);
        let measured = frame
            .fill_joint_radii_with_f32_array(&spaces, &radii)
            .unwrap_or(false);
        if !posed || !measured {
            // And the joint-at-a-time path is the one below, which is where a browser without these methods
            // ends up. Said at `debug` because it is the answer to "why is this hand slow", which is a
            // question asked while profiling rather than one a page needs every frame.
            log::debug!(
                "wxr-webxr: this hand has no bulk answer, so it is asked a joint at a time"
            );
            return false;
        }
        log::debug!("wxr-webxr: a whole hand in two calls");
        let (matrices, radii) = (matrices.to_vec(), radii.to_vec());
        for joint in wxr::HandJoint::ALL {
            let start = joint.index() * 16;
            let Some(pose) = wxr::Pose::from_matrix(&matrices[start..][..16]) else {
                // A slot the browser said it had filled and left as zeros, which is not a pose and so is a
                // joint that is not tracked - the same answer as an empty `Hand`.
                continue;
            };
            out.joints_mut()[joint.index()] = Some(wxr::Joint {
                pose,
                radius: radii[joint.index()],
            });
        }
        true
    }
}
