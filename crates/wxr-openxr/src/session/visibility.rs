//! What part of each eye's picture the display shows, which is `XR_KHR_visibility_mask`.
//!
//! The extension is a query and an event: the query answers with a triangle mesh on the view plane, and the
//! event says the mesh moved. Reading it once a frame would be reading a value that changes when the headset
//! shifts on the wearer's face rather than when the app asks, so each view's answer is kept until the runtime
//! says it changed - which is what [`VisibilityMasks::invalidate`] does, from the event.

use super::*;

/// The masks this session has read, by view index, and the extension's presence.
///
/// One per session and not one per frame, because the answer is about the headset rather than the frame. A
/// slot is `None` until that view has been asked about, and a mask with no geometry is stored like any other:
/// "the whole view is visible" is an answer worth keeping too.
pub(super) struct VisibilityMasks {
    masks: Vec<Option<wxr::VisibilityMask>>,
}

impl VisibilityMasks {
    /// Hold the masks of a runtime that was made with `XR_KHR_visibility_mask`, or say there are none.
    pub(super) fn new(enabled: bool) -> Option<Self> {
        if !enabled {
            log::info!(
                "wxr-openxr: no visibility masks: this runtime does not list XR_KHR_visibility_mask"
            );
            return None;
        }
        Some(Self { masks: Vec::new() })
    }

    /// Forget one view's mask, because the runtime said it changed - the next ask reads the new one.
    pub(super) fn invalidate(&mut self, view: usize) {
        if let Some(slot) = self.masks.get_mut(view) {
            *slot = None;
        }
    }

    /// The mask for one view, read now if it has not been read since the last change.
    pub(super) fn mask(
        &mut self,
        session: &xr::Session<xr::Vulkan>,
        view: usize,
    ) -> Result<wxr::VisibilityMask, wxr::Error> {
        if let Some(Some(mask)) = self.masks.get(view) {
            return Ok(mask.clone());
        }
        let index = u32::try_from(view)
            .map_err(|_| wxr::Error::Unsupported(format!("view {view} is not an index").into()))?;
        // The visible triangle mesh rather than the hidden one or the loop: it is what a renderer wants to
        // draw, and the other two would have to be inverted - which a stencil test can do but a mask handed
        // to an app cannot.
        let mask = session
            .get_visibility_mask_khr(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                index,
                xr::VisibilityMaskTypeKHR::VISIBLE_TRIANGLE_MESH,
            )
            .map_err(|error| {
                wxr::Error::Unsupported(
                    format!("the visibility mask for view {view}: {error:?}").into(),
                )
            })?;
        let mask = wxr::VisibilityMask {
            vertices: mask
                .vertices
                .iter()
                .map(|vertex| wxr::glam::Vec2::new(vertex.x, vertex.y))
                .collect(),
            indices: mask.indices,
        };
        if self.masks.len() <= view {
            self.masks.resize(view + 1, None);
        }
        self.masks[view] = Some(mask.clone());
        Ok(mask)
    }
}

impl OpenXrSession {
    pub(super) fn visibility_mask_impl(
        &mut self,
        view: usize,
        out: &mut wxr::VisibilityMask,
    ) -> Result<(), wxr::Error> {
        out.clear();
        // The two fields apart, because the mask's cache is written while the session it is asked through
        // is borrowed - which is one borrow of the session as a whole, and one too many.
        let Self {
            visibility_masks,
            session,
            ..
        } = self;
        let Some(visibility) = visibility_masks.as_mut() else {
            return Err(wxr::Error::Unsupported("visibility masks".into()));
        };
        *out = visibility.mask(session, view)?;
        Ok(())
    }
}
