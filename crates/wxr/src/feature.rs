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
