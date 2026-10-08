//! What a test asks a mock session afterwards.
//!
//! Every answer here is a question the core does not have - where a layer was placed, what a buzz was, how
//! many frames have gone by - because the core's own answers are about a session in the present and a test is
//! asking about the past. They live apart from the session itself so that the file that says what the mock
//! *is* is not the file a test reads to see what it did.

use super::*;

impl MockSession {
    /// How many frames have been begun, so a test can tell the loop ran.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The mode the session was asked for, which a mock can hold even though nothing acts on it.
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Pretend the primary action was pressed and released on `id`.
    ///
    /// A real runtime makes these events out of a controller; a test needs them without one, and they are the
    /// whole shape a game reacts to - a start, an end, and the selection that completed.
    pub fn press(&mut self, id: InputId) {
        self.pending.push_back(Event::SelectStart(id));
        self.pending.push_back(Event::SelectEnd(id));
        self.pending.push_back(Event::Select(id));
    }

    /// Pretend something was picked up or put down, which is the set of inputs changing.
    pub fn inputs_changed(&mut self) {
        self.pending.push_back(Event::InputsChanged);
    }

    /// Every vibration asked for, oldest first, as the source, the intensity and the length.
    pub fn pulses(&self) -> &[(InputId, f32, Duration)] {
        &self.pulses
    }

    /// Every waveform asked for, oldest first, as the source, how many samples, and the rate.
    pub fn waveforms(&self) -> &[(InputId, usize, f32)] {
        &self.waveforms
    }

    /// Pretend a space's origin was recentered, which is the one thing a backend that cannot recenter still
    /// has to be able to pass on.
    pub fn reset(&mut self, space: ReferenceSpace) {
        self.pending.push_back(Event::Reset(space));
    }

    /// How many layers are live, so a test can tell a release from a leak.
    pub fn layer_count(&self) -> usize {
        self.layers.iter().filter(|layer| layer.is_some()).count()
    }

    /// Where a layer was last placed, which is how a test sees that placing it did something.
    pub fn layer_pose(&self, layer: Layer) -> Option<Pose> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| slot.pose)
    }

    /// The shape and the resolution a layer was made with, which the core deliberately does not hand back: a
    /// caller that asked knows, and a test that asks is checking that it was kept.
    pub fn layer_shape(&self, layer: Layer) -> Option<(LayerShape, Extent2d)> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| (slot.shape, slot.pixels))
    }

    /// The space a layer was made in.
    pub fn layer_space(&self, layer: Layer) -> Option<ReferenceSpace> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| slot.space)
    }
}
