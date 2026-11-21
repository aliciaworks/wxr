//! Devices: what a controller is on this platform, where it is, and the one thing it can be asked to do.
//!
//! visionOS has controllers, and GameController is the framework that has them: a Bluetooth pad is a
//! `GCController`, and since visionOS 26 a spatial one - a PlayStation VR2 Sense pad - is too. What
//! the core asks of a source is answered here, and the answer that is this platform's alone is
//! [`wxr::InputSource::haptics`]: `GCDeviceHaptics` is the set of actuators, and a device whose set is
//! empty is a device that does not buzz.
//!
//! **A spatial stylus is the framework's other device.** It is a `GCStylus`, in a list of its own, and
//! since visionOS 26 it is a source here as well: its tip is the selection, the button nearest the tip is
//! the squeeze, and the pressure the tip measures is the trigger - see [`stylus`]. The rest of a stylus is
//! what it is for a pad: a place from ARKit, a product category, and haptics when it has an actuator.
//!
//! A device's place comes from ARKit's accessory tracking rather than GameController - see
//! `crate::arkit::accessory` - and the two are joined when the device is first seen, by loading an
//! accessory from the same `GCDevice`. A device ARKit has no accessory for keeps the place it always had:
//! [`wxr::InputSource::tracked`] is `false` and the poses are the identity.
//!
//! `play_pcm` is not here: Apple's haptics are patterns on an engine, not samples handed to a driver - see
//! [`haptics`].

use std::collections::VecDeque;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_haptics::{CHHapticEngine, CHHapticPatternPlayer, CHHapticTimeImmediate};
use objc2_game_controller::{
    GCController, GCControllerAxisInput, GCControllerButtonInput, GCDevice, GCDeviceHaptics,
    GCExtendedGamepad, GCHapticsLocalityDefault, GCStylus,
};

use crate::arkit::ArKit;
use crate::arkit::accessory::controller_place;

mod edges;
mod haptics;
mod stylus;

use edges::press_edges;
use haptics::pattern;

/// Where a device's id starts, so that a hand and a device can never be the same number.
///
/// The core's [`wxr::InputId`] is a `u32` with no namespace, and ARKit's hands are numbered from zero - so
/// the offset is what keeps the two lists from meeting. It is a constant rather than a count because the
/// number of hands changes as they come in and out of view, and an id that moves is worse than a gap.
///
/// `pub` for the same reason it exists: `session::sources` reads a hand's id back as an index into the two
/// hands, and that reading is only safe if this offset is the thing the two halves agree on. Two copies of
/// this number would compile and then disagree on a device.
pub const CONTROLLER_ID_BASE: u32 = 1024;

/// The devices this session has seen, and the engines their buzzes play on.
pub struct GameControllers {
    known: Vec<Known>,
    /// The press and squeeze edges the last read found, waiting to be polled.
    pending: VecDeque<wxr::Event>,
}

struct Known {
    /// Retained because the framework's array is not ours: a device outlives the frame that found it, and
    /// the object is the same one every frame, which is what the ids are keyed on.
    device: Device,
    id: wxr::InputId,
    /// The engine a buzz for this device plays on, made when one is first sent and kept afterwards: a
    /// pattern is played *on* an engine, and an engine dropped between two pulses is the thing that was asked
    /// to buzz, dropped.
    engine: Option<Retained<CHHapticEngine>>,
    /// Whether it was connected the last time the framework was asked. Kept so that a device going away
    /// is a change too: a list that only noticed arrivals would leave the app holding a source that is gone.
    connected: bool,
    /// What its buttons were the last time they were read, so a press is an edge and not a state.
    buttons_was: wxr::Buttons,
    /// The player of the last pulse, kept because Core Haptics does not retain a player for the length of
    /// its pattern: dropping it is stopping it. A new pulse replaces the old one.
    player: Option<Retained<ProtocolObject<dyn CHHapticPatternPlayer>>>,
}

/// The two kinds of `GCDevice` the framework hands out.
///
/// They are separate `NSObject`s with separate lists - a stylus is not a controller and a controller is not
/// a stylus - but both conform to `GCDevice`, which is the protocol ARKit's accessory load takes and the
/// protocol a product category is asked of. Everything a source needs that is not a button is therefore the
/// same question of either, and only the profile the buttons are read from differs.
#[derive(Clone)]
enum Device {
    Controller(Retained<GCController>),
    /// A spatial stylus, visionOS 26 and later.
    Stylus(Retained<GCStylus>),
}

impl Device {
    /// The same device as the protocol both kinds conform to.
    fn as_gcdevice(&self) -> &ProtocolObject<dyn GCDevice> {
        match self {
            Self::Controller(controller) => ProtocolObject::from_ref(&**controller),
            Self::Stylus(stylus) => ProtocolObject::from_ref(&**stylus),
        }
    }

    /// The actuators this device can be buzzed on, if it has any.
    fn haptics(&self) -> Option<Retained<GCDeviceHaptics>> {
        // SAFETY: both properties are the framework's own answers about a device it handed out.
        unsafe {
            match self {
                Self::Controller(controller) => controller.haptics(),
                Self::Stylus(stylus) => stylus.haptics(),
            }
        }
    }

    /// The core's buttons and axes, from whichever profile the device reports them on.
    fn inputs(&self) -> (wxr::Buttons, wxr::Axes) {
        match self {
            Self::Controller(controller) => {
                // SAFETY: the controller is live, and the profile is the framework's answer about it.
                let profile = unsafe { controller.extendedGamepad() };
                (buttons(profile.as_deref()), axes(profile.as_deref()))
            }
            Self::Stylus(stylus) => {
                // SAFETY: the stylus is live, and this is the input profile the framework reports for it.
                let input = unsafe { stylus.input() };
                stylus::read(input.as_deref())
            }
        }
    }

    /// Whether this is the same device as `other`: object identity, not a comparison of what it reports.
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Controller(a), Self::Controller(b)) => a == b,
            (Self::Stylus(a), Self::Stylus(b)) => a == b,
            _ => false,
        }
    }
}

impl GameControllers {
    pub fn new() -> Self {
        Self {
            known: Vec::new(),
            pending: VecDeque::new(),
        }
    }

    /// The next press or release this framework saw, if one is waiting.
    pub fn poll(&mut self) -> Option<wxr::Event> {
        self.pending.pop_front()
    }

    /// Every device connected right now, as the core's sources, and whether the set changed.
    ///
    /// The ids are the order the session first saw them in, which is what makes one stable while a device
    /// stays connected: the framework hands back the same object every frame, so identity is what the list
    /// is keyed on. A device that goes away keeps its entry, and its id stays spent even if it never comes
    /// back - handing the number to a different device would make a press from somewhere else look like
    /// the same one.
    ///
    /// `arkit` gives a device its place, and `space_from_world` brings that into the caller's space. `at` is
    /// the Mach absolute time ARKit should predict that place for - the frame's trackable anchor time - and
    /// `None` leaves it at the latest anchor.
    pub fn read(
        &mut self,
        mut arkit: Option<&mut ArKit>,
        at: Option<f64>,
        space_from_world: wxr::glam::Mat4,
        out: &mut Vec<wxr::InputSource>,
    ) -> bool {
        // SAFETY: the framework is live, and this is its own list of what is connected to it.
        let mut connected: Vec<Device> = unsafe { GCController::controllers() }
            .iter()
            .map(|controller| Device::Controller(controller.clone()))
            .collect();
        // A stylus is not a controller, so it comes from a list of its own - and one that only exists from
        // visionOS 26, which is why the class method is asked behind the availability check.
        if objc2::available!(visionos = 26.0) {
            // SAFETY: the class and its method exist on every system past the guard, and this is the
            // framework's own list of what is connected.
            connected.extend(
                unsafe { GCStylus::styli() }
                    .iter()
                    .map(|stylus| Device::Stylus(stylus.clone())),
            );
        }
        let mut changed = false;
        let mut edges = Vec::new();
        for device in &connected {
            if !self.known.iter().any(|known| known.device.same(device)) {
                let id = wxr::InputId::new(CONTROLLER_ID_BASE + self.known.len() as u32);
                // Asked once, when the device is first seen: a load is a request rather than a state.
                if let Some(arkit) = arkit.as_deref_mut() {
                    arkit.load_accessory(device.as_gcdevice());
                }
                self.known.push(Known {
                    device: device.clone(),
                    id,
                    engine: None,
                    connected: true,
                    buttons_was: wxr::Buttons::default(),
                    player: None,
                });
                changed = true;
            }
        }
        // Asked after the loads above, so a device first seen now is only a frame behind the load.
        let places = arkit
            .map(|arkit| arkit.accessory_poses(at))
            .unwrap_or_default();
        for known in &mut self.known {
            let here = connected.iter().any(|device| known.device.same(device));
            if known.connected != here {
                known.connected = here;
                changed = true;
            }
            // A device that has gone away is not reported, and its id stays spent: handing it to a
            // different device would make a press that came from somewhere else look like the same one.
            if !here {
                // A device that is gone has no buttons, and a press held when it left must not look
                // like one already released when it comes back.
                known.buttons_was = wxr::Buttons::default();
                continue;
            }
            let (buttons, axes) = known.device.inputs();
            press_edges(known.id, known.buttons_was, buttons, &mut edges);
            known.buttons_was = buttons;
            // The anchor ARKit reports names the `GCDevice` it came from, which is this device.
            let place = controller_place(&places, known.device.as_gcdevice(), space_from_world);
            let haptics = known.device.haptics().is_some();
            out.push(wxr::InputSource {
                id: known.id,
                // The hand holding it, or the one it is designed for when it is set down - see
                // `crate::arkit::accessory`.
                handedness: place.handedness,
                target_ray_mode: wxr::TargetRayMode::TrackedPointer,
                // No skeleton.
                hand: false,
                haptics,
                // GameController draws no device, so a model of it is the app's.
                skip_rendering: false,
                // Whether that place is a guess - orientation without position - is the anchor's answer.
                emulated_position: place.emulated_position,
                // ARKit's own estimate of how fast it moves, or nothing when it has no anchor - see
                // `crate::arkit::accessory`.
                motion: place.motion,
                // Whether a hand is holding it, which is ARKit's answer and not the generic tracked one: a
                // pad on a table is tracked and not held - see `crate::arkit::accessory`.
                held: place.held,
                // Two poses: the accessory's grip and aim locations - see `crate::arkit::accessory`.
                grip: place.grip,
                aim: place.aim,
                tracked: place.tracked,
                buttons,
                axes,
            });
        }
        self.pending.extend(edges);
        changed
    }

    /// The name the framework gives a device, which is its product category.
    ///
    /// `GCDevice.productCategory` is a string like `"DualSense"`, `"Xbox One"` or `"SpatialStylus"`, and it is
    /// the only name a device has here: it is what an app switches on to draw the right picture, and it does
    /// not change while a device stays connected. A device that has gone away keeps its id but answers with
    /// nothing, which is the same as one this session never handed out.
    pub fn profiles(&self, id: wxr::InputId, out: &mut Vec<String>) {
        out.clear();
        let Some(known) = self.known.iter().find(|known| known.id == id) else {
            return;
        };
        if !known.connected {
            return;
        }
        // SAFETY: the device is live, and `GCDevice` is a protocol it conforms to - the property is the
        // framework's own answer about its own device.
        out.push(unsafe { known.device.as_gcdevice().productCategory() }.to_string());
    }

    /// Whether any connected device has an actuator to send to, which is the capability bit.
    ///
    /// Asked of the devices rather than remembered from the last frame, because a bit that lags a connection
    /// by a frame is a bit an app can draw the wrong thing from.
    pub fn has_haptics(&self) -> bool {
        // SAFETY: the framework is live and these are its own questions about its own devices. The stylus
        // list only exists from visionOS 26, so it is asked behind the availability check.
        let controllers = unsafe { GCController::controllers() }
            .iter()
            .any(|controller| unsafe { controller.haptics() }.is_some());
        let styli = objc2::available!(visionos = 26.0)
            && unsafe { GCStylus::styli() }
                .iter()
                .any(|stylus| unsafe { stylus.haptics() }.is_some());
        controllers || styli
    }

    /// Send a vibration to one device, as a Core Haptics pattern on its own engine.
    pub fn pulse(
        &mut self,
        id: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        let Some(known) = self.known.iter_mut().find(|known| known.id == id) else {
            return Err(wxr::Error::Unsupported("haptics on this source".into()));
        };
        // The entry outlives the connection, because its id has to stay spent - but a device that has
        // gone away is not one there is anything left to buzz.
        if !known.connected {
            return Err(wxr::Error::Unsupported(
                "haptics on a device that is gone".into(),
            ));
        }
        // The player is kept in the entry after this block: the engine is borrowed to make it, and Core
        // Haptics does not hold it for the length of the pattern.
        let player = {
            let engine = match &mut known.engine {
                Some(engine) => &*engine,
                None => {
                    // SAFETY: the device is live and this is the framework's own question about it.
                    let haptics = known
                        .device
                        .haptics()
                        .ok_or_else(|| wxr::Error::Unsupported("haptics on this source".into()))?;
                    // SAFETY: the default locality is the one GameController guarantees every device with
                    // an actuator has, and the engine is the framework's to make.
                    let engine =
                        unsafe { haptics.createEngineWithLocality(GCHapticsLocalityDefault) }
                            .ok_or_else(|| wxr::Error::Unsupported("a haptic engine".into()))?;
                    known.engine.insert(engine)
                }
            };
            // SAFETY: the engine is this device's and is live. Starting one that is already running is
            // the framework's own no-op, which is why this is not guarded by a flag of ours.
            unsafe { engine.startAndReturnError() }
                .map_err(|error| wxr::Error::Rejected(format!("{error}").into()))?;
            pattern(engine, intensity, duration)?
        };
        // SAFETY: the player is live, and `CHHapticTimeImmediate` is the framework's "as soon as possible".
        unsafe { player.startAtTime_error(CHHapticTimeImmediate) }
            .map_err(|error| wxr::Error::Rejected(format!("{error}").into()))?;
        // Kept for the duration of the pattern; without it the player is dropped after the call that starts
        // it, which stops the thing that was asked to buzz.
        known.player = Some(player);
        Ok(())
    }

    /// Stop the buzz a device is playing, if it is playing one - `CHHapticPatternPlayer.stopAtTime:`.
    ///
    /// The player is the object the buzz was started on, so stopping it is the same one asked the other
    /// question, and the pattern ends where it is rather than being dropped: a player released mid-pattern
    /// *is* the stop, but it is the allocator's answer rather than one the framework gives an error for.
    ///
    /// A device with no player is `Ok` - it is not buzzing, which is what was asked for - and the other
    /// two refusals are [`pulse`](GameControllers::pulse)'s: an id this session never had, and a device
    /// that has gone away since.
    pub fn stop(&mut self, id: wxr::InputId) -> Result<(), wxr::Error> {
        let Some(known) = self.known.iter_mut().find(|known| known.id == id) else {
            return Err(wxr::Error::Unsupported("haptics on this source".into()));
        };
        if !known.connected {
            return Err(wxr::Error::Unsupported(
                "haptics on a device that is gone".into(),
            ));
        }
        let Some(player) = known.player.take() else {
            return Ok(());
        };
        // SAFETY: the player is live, and `CHHapticTimeImmediate` is the framework's "as soon as possible" -
        // the same time the buzz was started with, which is what ending it now means.
        unsafe { player.stopAtTime_error(CHHapticTimeImmediate) }
            .map_err(|error| wxr::Error::Rejected(format!("{error}").into()))
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
