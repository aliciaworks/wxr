//! Places the runtime keeps fixed in the room, which is WebXR's Anchors module.

/// A place the runtime is keeping fixed relative to the world, which is WebXR's `XRAnchor`.
///
/// A handle and not a pose, which is the whole point of the module: the runtime re-tracks where the place is as
/// its understanding of the room changes, and what an app holds is the name to ask by. The same shape
/// [`crate::ReferenceSpace`] has, and for the same reason - a caller that cached a pose would be holding a
/// stale one, and a stale anchor is exactly what this exists to avoid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Anchor {
    id: u32,
}

impl Anchor {
    /// An anchor a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}
