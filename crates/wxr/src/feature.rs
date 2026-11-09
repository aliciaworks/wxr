//! What a session can do beyond what every session can.

/// A set of the things a session can do, which is `wgpu`'s `Features` and WebXR's `enabledFeatures` - and the
/// reason both settled on the same shape is the same: an app that has to discover what a runtime has by *trying*
/// every call is an app that starts by guessing. Asking once and branching is what a capability is for.
///
/// What every one of these describes is a part of the core that is optional *and says so plainly*: a session
/// without it answers `None`, an empty list, or `Error::Unsupported` from the matching method. So the set is
/// not how an app avoids a crash - it is how an app knows what to offer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Features(u32);

impl Features {
    /// The session itself, which is every session.
    pub const NONE: Self = Self(0);

    /// What the runtime measured of the real world per eye: [`crate::Session::depth`].
    pub const DEPTH: Self = Self(1 << 0);
    /// The surfaces a runtime has found: [`crate::Session::planes`].
    pub const PLANES: Self = Self(1 << 1);
    /// Where a ray out of a space meets the world: [`crate::Session::hit_test_source`].
    pub const HIT_TEST: Self = Self(1 << 2);
    /// The room's light: [`crate::Session::light_probe`].
    pub const LIGHT_ESTIMATION: Self = Self(1 << 3);
    /// A hand's skeleton: [`crate::Session::hand`]. A hand's *place* is not this - that is an input source like
    /// any other, and comes without asking.
    pub const HAND_TRACKING: Self = Self(1 << 4);
    /// Places the runtime keeps fixed in the room: [`crate::Session::anchor`].
    pub const ANCHORS: Self = Self(1 << 5);

    /// A flat rectangle the compositor places: [`crate::Session::layer`] with [`crate::LayerShape::Quad`].
    pub const LAYER_QUAD: Self = Self(1 << 6);
    /// A rectangle bent around a vertical cylinder: [`crate::LayerShape::Cylinder`].
    pub const LAYER_CYLINDER: Self = Self(1 << 7);
    /// A sphere's worth of picture: [`crate::LayerShape::Equirect`].
    pub const LAYER_EQUIRECT: Self = Self(1 << 8);
    /// Six faces of a cube in one image: [`crate::LayerShape::Cube`].
    pub const LAYER_CUBE: Self = Self(1 << 9);

    /// The display's refresh rate can be asked for - `XR_FB_display_refresh_rate` on a Quest, and its like
    /// elsewhere. Not a WebXR member: the browser picks the rate and a page cannot, so this is one of the
    /// places the core is wider than the specification it is named after, the way [`crate::Presentation`] is.
    pub const REFRESH_RATE: Self = Self(1 << 10);

    /// A source that can be made to buzz: [`crate::Session::pulse`] and [`crate::Session::play_pcm`].
    ///
    /// A bit of its own rather than something a caller reads off a source, because whether *this* session has
    /// any is a question an app asks before it decides what to offer - and because a platform can have the
    /// hardware and no way to reach it. What is per-source is [`crate::InputSource::haptics`], which says
    /// which of the things in the list can be felt; this says there is a list at all.
    pub const HAPTICS: Self = Self(1 << 11);

    /// The room traced as triangles: [`crate::Session::meshes`].
    ///
    /// Not a finer [`PLANES`](Features::PLANES): a runtime can have either without the other, and ARKit is
    /// the case in point - scene reconstruction traces meshes, and its planes come from a provider of their
    /// own that need not be on at all.
    pub const MESH: Self = Self(1 << 12);

    /// Whether every bit of `other` is in this set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether there is anything here beyond the session itself.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// A set with everything that is in either, which is how a backend collects what it got.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
