//! What this session can do, as the core's [`wxr::Features`] - the bits ARKit's providers and GameController
//! between them decide.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the body lives here so that the
//! dispatch stays a list of answers rather than becoming the place every list is built.

use super::*;

impl AppleSession {
    /// The bits this session has: one per provider that came up, and the wearer's permission asked where a
    /// provider serves a room capability rather than the wearer.
    pub(super) fn features_impl(&self) -> wxr::Features {
        // ARKit's world tracking is a reference space rather than a capability. The surfaces it does have
        // are the providers below - one bit each, because a session can have one without the other.
        let mut features = wxr::Features::NONE;
        if self.arkit.as_ref().is_some_and(ArKit::has_planes) {
            features = features.union(wxr::Features::PLANES);
        }
        if self.arkit.as_ref().is_some_and(ArKit::has_anchors) {
            features = features.union(wxr::Features::ANCHORS);
        }
        // Meshes, and the bit is not the planes one: ARKit's scene reconstruction is its own provider, and a
        // session can trace the room as triangles without the plane provider being up at all.
        if self.arkit.as_ref().is_some_and(ArKit::has_meshes) {
            features = features.union(wxr::Features::MESH);
        }
        // Hit testing, which is neither provider: a ray is cast against whichever shape of the room came up,
        // so either one is enough and the bit says the cast can be made.
        if self.arkit.as_ref().is_some_and(ArKit::has_hit_test) {
            features = features.union(wxr::Features::HIT_TEST);
        }
        // Light estimation, which is ARKit's environment probe rather than either shape provider above: a
        // session can trace a room as triangles and still be unable to photograph it, and the bit is the
        // provider being up rather than a picture having arrived - a room too dark to photograph answers
        // with no intensity and zero harmonics, which the core says means nothing was measured.
        if self.arkit.as_ref().is_some_and(ArKit::has_light) {
            features = features.union(wxr::Features::LIGHT_ESTIMATION);
        }
        // Device fit, and the one bit about the wearer rather than the room: a session can track a room
        // perfectly with its optics out of line, and the provider being up is all this can promise.
        if self.arkit.as_ref().is_some_and(ArKit::has_device_fit) {
            features = features.union(wxr::Features::DEVICE_FIT);
        }
        // Hand tracking, which is the provider rather than the skeleton: a device whose hand provider came up
        // is a device that reports hands, and whether a joint is tracked this frame is what `Session::hand`
        // answers per joint. A hand ARKit has lost is empty, which is what the bit warns about.
        if self.arkit.as_ref().is_some_and(ArKit::has_hands) {
            features = features.union(wxr::Features::HAND_TRACKING);
        }
        // Haptics is the one bit here that does not come from ARKit: a controller is GameController's, and
        // whether a connected one can be felt is that framework's answer rather than the platform's.
        if self.controllers.has_haptics() {
            features = features.union(wxr::Features::HAPTICS);
        }
        features
    }
}
