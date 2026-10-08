//! What the session itself says: how it reads input, what depth sensing it has, and which features it
//! was granted.
//!
//! These are answers about the session rather than about a frame, which is why they are not in `frames`
//! or `events`. Two of them are questions only a browser can answer - the input of a compositor's session is
//! tracked in the scene by definition, and its depth is a buffer to draw into rather than a measurement the
//! app asked for - and the third is this backend's own reading of what the browser granted.

use super::*;

impl WebXrSession {
    pub(super) fn interaction_mode_impl(&self) -> wxr::InteractionMode {
        let Some(session) = self.inner.borrow().session.clone() else {
            return wxr::InteractionMode::WorldSpace;
        };
        match session.interaction_mode() {
            XrInteractionMode::WorldSpace => wxr::InteractionMode::WorldSpace,
            XrInteractionMode::ScreenSpace => wxr::InteractionMode::ScreenSpace,
            // A browser that says something this build has never heard of: the world is where the input of a
            // session drawn into a display is, and it is the answer for every other backend too.
            XrInteractionMode::__Invalid => wxr::InteractionMode::WorldSpace,
        }
    }
}

impl WebXrSession {
    pub(super) fn depth_sensing_impl(&self) -> Option<wxr::DepthSensing> {
        let session = self.inner.borrow().session.clone()?;
        Some(wxr::DepthSensing {
            usage: match session.depth_usage() {
                XrDepthUsage::CpuOptimized => wxr::DepthUsage::CpuOptimized,
                XrDepthUsage::GpuOptimized => wxr::DepthUsage::GpuOptimized,
                XrDepthUsage::__Invalid => wxr::DepthUsage::GpuOptimized,
            },
            format: match session.depth_data_format() {
                XrDepthDataFormat::LuminanceAlpha => wxr::DepthFormat::LuminanceAlpha,
                XrDepthDataFormat::Float32 => wxr::DepthFormat::Float32,
                XrDepthDataFormat::UnsignedShort => wxr::DepthFormat::UnsignedShort,
                XrDepthDataFormat::__Invalid => wxr::DepthFormat::Float32,
            },
            ty: session.depth_type().map(|ty| match ty {
                XrDepthType::Raw => wxr::DepthType::Raw,
                XrDepthType::Smooth => wxr::DepthType::Smooth,
                XrDepthType::__Invalid => wxr::DepthType::Raw,
            }),
            active: session.depth_active(),
        })
    }
}

impl WebXrSession {
    pub(super) fn features_impl(&self) -> wxr::Features {
        let Some(session) = self.inner.borrow().session.clone() else {
            // Before the session arrives there is nothing to ask, and an answer that is not here yet is not a
            // capability.
            return wxr::Features::NONE;
        };
        let mut features = wxr::Features::NONE;
        for (bit, name) in [
            (wxr::Features::PLANES, "plane-detection"),
            (wxr::Features::HIT_TEST, "hit-test"),
            (wxr::Features::LIGHT_ESTIMATION, "light-estimation"),
            (wxr::Features::HAND_TRACKING, "hand-tracking"),
            (wxr::Features::ANCHORS, "anchors"),
        ] {
            if has_feature(&session, name) {
                features = features.union(bit);
            }
        }
        // Depth is granted by a feature and readable only through the binding that gives a texture: a browser
        // that granted it and no binding is a depth this backend cannot hand over.
        if has_feature(&session, "depth-sensing") && self.gpu.is_some() {
            features = features.union(wxr::Features::DEPTH);
        }
        // Layers need two things and the session has to have both: the binding, which is what makes one, and the
        // feature descriptor, which is what lets a non-projection layer be composited at all.
        if self.gpu.is_some() && has_feature(&session, "layers") {
            features = features.union(wxr::Features::LAYER_QUAD);
        }
        features
    }
}
