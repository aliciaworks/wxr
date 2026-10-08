//! Gamepads: what a controller is on this platform, and the one thing it can be asked to do.
//!
//! visionOS has controllers, and GameController is the framework that has them: a Bluetooth pad is a
//! `GCController`, and since visionOS 26 a spatial one - a PlayStation VR2 Sense pad is one - is too. What
//! the core asks of a source is answered here, and the answer that is this platform's alone is
//! [`wxr::InputSource::haptics`]: `GCDeviceHaptics` is the set of actuators, and a controller whose set is
//! empty is a controller that does not buzz.
//!
//! Only the buzz is here. Where a controller is in the room comes from ARKit's accessory tracking rather than
//! from GameController - `ar_accessory_anchor_*`, which is the next piece - so a controller here is a device
//! with buttons and an actuator and no place, which is exactly what a plain Bluetooth gamepad is.
//! [`wxr::InputSource::tracked`] is `false` and the poses are the identity, and neither is a placeholder:
//! they are what "a gamepad" means. A spatial accessory gets a place the day the ARKit half is written, and
//! then this one says `true`.
//!
//! `play_pcm` is not here. Apple's haptics are patterns played on an engine rather than samples handed to a
//! driver, so there is no waveform call for this platform to fill - the shape the core has for one is
//! OpenXR's and the haptics draft's, and both of them are recorded the same way.

use std::time::Duration;

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_haptics::{
    CHHapticEngine, CHHapticEvent, CHHapticEventParameter, CHHapticEventParameterIDHapticIntensity,
    CHHapticEventTypeHapticContinuous, CHHapticPattern, CHHapticPatternPlayer,
    CHHapticTimeImmediate,
};
use objc2_foundation_visionos::NSArray;
use objc2_game_controller::{
    GCController, GCControllerAxisInput, GCControllerButtonInput, GCExtendedGamepad,
    GCHapticsLocalityDefault,
};

/// Where a controller's id starts, so that a hand and a controller can never be the same number.
///
/// The core's [`wxr::InputId`] is a `u32` with no namespace, and ARKit's hands are numbered from zero - so
/// the offset is what keeps the two lists from meeting. It is a constant rather than a count because the
/// number of hands changes as they come in and out of view, and an id that moves is worse than a gap.
const CONTROLLER_ID_BASE: u32 = 1024;

/// The controllers this session has seen, and the engines their buzzes play on.
pub struct GameControllers {
    known: Vec<Known>,
}

struct Known {
    /// Retained because the framework's array is not ours: a controller outlives the frame that found it, and
    /// the object is the same one every frame, which is what the ids are keyed on.
    controller: Retained<GCController>,
    id: wxr::InputId,
    /// The engine a buzz for this controller plays on, made when one is first sent and kept afterwards: a
    /// pattern is played *on* an engine, and an engine dropped between two pulses is the thing that was asked
    /// to buzz, dropped.
    engine: Option<Retained<CHHapticEngine>>,
}

impl GameControllers {
    pub fn new() -> Self {
        Self { known: Vec::new() }
    }

    /// Every controller connected right now, as the core's sources, and whether a new one appeared.
    ///
    /// The ids are the order the session first saw them in, which is what makes one stable while a controller
    /// stays connected: `GCController` hands back the same object every frame, so identity is what the list is
    /// keyed on and there is no second name to invent.
    pub fn read(&mut self, out: &mut Vec<wxr::InputSource>) -> bool {
        // SAFETY: the framework is live, and this is its own list of what is connected to it.
        let connected = unsafe { GCController::controllers() }.to_vec();
        let before = self.known.len();
        for controller in &connected {
            if !self
                .known
                .iter()
                .any(|known| known.controller == *controller)
            {
                let id = wxr::InputId::new(CONTROLLER_ID_BASE + self.known.len() as u32);
                self.known.push(Known {
                    controller: controller.clone(),
                    id,
                    engine: None,
                });
            }
        }
        for known in &self.known {
            // A controller that has gone away is not reported, and its id stays spent: handing it to a
            // different controller would make a press that came from somewhere else look like the same one.
            if !connected.contains(&known.controller) {
                continue;
            }
            // SAFETY: a controller from the framework's own list, and its profile and its haptics are the
            // framework's answers about it.
            let (profile, haptics) = unsafe {
                (
                    known.controller.extendedGamepad(),
                    known.controller.haptics(),
                )
            };
            out.push(wxr::InputSource {
                id: known.id,
                // A pad has no handedness: it is held in two hands, and which half a button is on is the
                // profile's business rather than the core's.
                handedness: wxr::Handedness::Unknown,
                target_ray_mode: wxr::TargetRayMode::TrackedPointer,
                // No skeleton.
                hand: false,
                haptics: haptics.is_some(),
                // No place, because a gamepad has none - see the module comment.
                grip: wxr::Pose::IDENTITY,
                aim: wxr::Pose::IDENTITY,
                tracked: false,
                buttons: buttons(profile.as_deref()),
                axes: axes(profile.as_deref()),
            });
        }
        self.known.len() != before
    }

    /// Whether any connected controller has an actuator to send to, which is the capability bit.
    ///
    /// Asked of the controllers rather than remembered from the last frame, because a bit that lags a
    /// connection by a frame is a bit an app can draw the wrong thing from.
    pub fn has_haptics(&self) -> bool {
        // SAFETY: the framework is live and this is its own question about its own controller.
        unsafe { GCController::controllers() }
            .iter()
            .any(|controller| unsafe { controller.haptics() }.is_some())
    }

    /// Send a vibration to one controller, as a Core Haptics pattern on its own engine.
    pub fn pulse(
        &mut self,
        id: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        let Some(known) = self.known.iter_mut().find(|known| known.id == id) else {
            return Err(wxr::Error::Unsupported("haptics on this source".into()));
        };
        let engine = match &mut known.engine {
            Some(engine) => &*engine,
            None => {
                // SAFETY: the controller is live and this is the framework's own question about it.
                let haptics = unsafe { known.controller.haptics() }
                    .ok_or_else(|| wxr::Error::Unsupported("haptics on this source".into()))?;
                // SAFETY: the default locality is the one GameController guarantees every device with an
                // actuator has, and the engine is the framework's to make.
                let engine = unsafe { haptics.createEngineWithLocality(GCHapticsLocalityDefault) }
                    .ok_or_else(|| wxr::Error::Unsupported("a haptic engine".into()))?;
                known.engine.insert(engine)
            }
        };
        // SAFETY: the engine is this controller's and is live. Starting one that is already running is the
        // framework's own no-op, which is why this is not guarded by a flag of ours.
        unsafe { engine.startAndReturnError() }
            .map_err(|error| wxr::Error::Rejected(format!("{error}")))?;
        let player = pattern(engine, intensity, duration)?;
        // SAFETY: the player is live, and `CHHapticTimeImmediate` is the framework's "as soon as possible".
        unsafe { player.startAtTime_error(CHHapticTimeImmediate) }
            .map_err(|error| wxr::Error::Rejected(format!("{error}")))?;
        Ok(())
    }
}

/// Build the pattern for one pulse and hand back the player that will play it.
///
/// One continuous event whose only parameter is its intensity: the core's word is an intensity and a length,
/// and a continuous event with a duration is exactly that. `HapticSharpness` is left out on purpose - it is
/// the character of a feel rather than its strength, and a core word for it would be one only this platform
/// has a name for.
fn pattern(
    engine: &CHHapticEngine,
    intensity: f32,
    duration: Duration,
) -> Result<Retained<ProtocolObject<dyn CHHapticPatternPlayer>>, wxr::Error> {
    // SAFETY: every object below is the framework's, made here and dropped at the end of the call, and the
    // parameter id and event type are constants.
    unsafe {
        let parameter = CHHapticEventParameter::initWithParameterID_value(
            CHHapticEventParameter::alloc(),
            CHHapticEventParameterIDHapticIntensity,
            intensity.clamp(0.0, 1.0),
        );
        let event = CHHapticEvent::initWithEventType_parameters_relativeTime_duration(
            CHHapticEvent::alloc(),
            CHHapticEventTypeHapticContinuous,
            &NSArray::from_retained_slice(std::slice::from_ref(&parameter)),
            0.0,
            duration.as_secs_f64(),
        );
        let pattern = CHHapticPattern::initWithEvents_parameters_error(
            CHHapticPattern::alloc(),
            &NSArray::from_retained_slice(std::slice::from_ref(&event)),
            &NSArray::from_retained_slice(&[]),
        )
        .map_err(|error| wxr::Error::Rejected(format!("{error}")))?;
        engine
            .createPlayerWithPattern_error(&pattern)
            .map_err(|error| wxr::Error::Rejected(format!("{error}")))
    }
}

/// The core's three buttons, from the standard extended profile.
///
/// A controller with no extended profile - a micro gamepad, a remote - has none of them and reports none,
/// which is a controller this core can say nothing about rather than an error.
fn buttons(profile: Option<&GCExtendedGamepad>) -> wxr::Buttons {
    let Some(profile) = profile else {
        return wxr::Buttons::default();
    };
    // SAFETY: every input below belongs to the profile and is live for as long as it is.
    unsafe {
        let shoulder = pressed(&profile.leftShoulder()) || pressed(&profile.rightShoulder());
        wxr::Buttons {
            // The main button is the one under the thumb, which is what a game means by "the" button.
            select: pressed(&profile.buttonA()),
            // The grip is what a hand closes around, which on a pad with a handle each is the shoulders.
            squeeze: shoulder,
            menu: pressed(&profile.buttonMenu()),
        }
    }
}

/// How far the trigger is pulled, and where the stick is, from the standard extended profile.
fn axes(profile: Option<&GCExtendedGamepad>) -> wxr::Axes {
    let Some(profile) = profile else {
        return wxr::Axes::default();
    };
    // SAFETY: as in `buttons`.
    unsafe {
        let stick = profile.leftThumbstick();
        wxr::Axes {
            // Two triggers and one number: the core's trigger is the pull, and a pad that has two is a pad
            // where either of them can be the one being pulled.
            trigger: button_value(&profile.leftTrigger())
                .max(button_value(&profile.rightTrigger())),
            thumbstick: wxr::glam::Vec2::new(
                axis_value(&stick.xAxis()),
                axis_value(&stick.yAxis()),
            ),
        }
    }
}

/// Whether a button is down.
///
/// # Safety
///
/// The button must be an input of a live profile.
unsafe fn pressed(button: &GCControllerButtonInput) -> bool {
    unsafe { button.isPressed() }
}

/// How far a trigger is pulled, `0..=1`.
///
/// # Safety
///
/// The button must be an input of a live profile.
unsafe fn button_value(button: &GCControllerButtonInput) -> f32 {
    unsafe { button.value() }
}

/// Where an axis is, `-1..=1`.
///
/// # Safety
///
/// The axis must be an input of a live profile.
unsafe fn axis_value(axis: &GCControllerAxisInput) -> f32 {
    unsafe { axis.value() }
}
