//! What part of each eye's picture the browser says the display shows, which is WebXR's visibility mask.
//!
//! The specification puts it behind an event rather than a query: a browser dispatches `visibilitymaskchange`
//! when the mask moves, and the event carries the new geometry. The attribute and the event class are newer
//! than the rest of the API, so the handler is installed only where the object has the attribute - a browser
//! without it is one whose whole views are visible.

use super::*;

impl WebXrSession {
    /// Install the `visibilitymaskchange` handler, when this browser has the attribute to install it on.
    pub(super) fn listen_for_visibility_masks(&mut self, session: &XrSession) {
        let has_attribute = js_sys::Reflect::has(
            session.as_ref(),
            &JsValue::from_str("onvisibilitymaskchange"),
        )
        .unwrap_or(false);
        if !has_attribute {
            log::info!(
                "wxr-webxr: no visibility masks: this browser has no onvisibilitymaskchange"
            );
            return;
        }

        let masks = self.visibility_masks.clone();
        let changed = self.visibility_mask_changed.clone();
        let handler = Closure::<dyn FnMut(XrVisibilityMaskChangeEvent)>::new(
            move |event: XrVisibilityMaskChangeEvent| {
                let view = XrVisibilityMaskChangeEvent::index(&event) as usize;
                let flat = XrVisibilityMaskChangeEvent::vertices(&event);
                let mask = wxr::VisibilityMask {
                    // The attribute is a flat list of X, Y pairs, which is what an `XRView`'s mask is: a
                    // vertex on the plane one metre ahead, so there is no Z to read.
                    vertices: flat
                        .chunks(2)
                        .filter_map(|pair| match pair {
                            [x, y] => Some(wxr::glam::Vec2::new(*x, *y)),
                            _ => None,
                        })
                        .collect(),
                    indices: XrVisibilityMaskChangeEvent::indices(&event),
                };
                let mut masks = masks.borrow_mut();
                if masks.len() <= view {
                    masks.resize(view + 1, None);
                }
                masks[view] = Some(mask);
                changed.borrow_mut().push_back(view);
            },
        );
        let set = js_sys::Reflect::set(
            session.as_ref(),
            &JsValue::from_str("onvisibilitymaskchange"),
            handler.as_ref(),
        );
        match set {
            Ok(true) => self.on_visibility_mask = Some(handler),
            _ => log::info!("wxr-webxr: no visibility masks: the handler would not attach"),
        }
    }

    pub(super) fn visibility_mask_impl(
        &mut self,
        view: usize,
        out: &mut wxr::VisibilityMask,
    ) -> Result<(), wxr::Error> {
        out.clear();
        if self.on_visibility_mask.is_none() {
            return Err(wxr::Error::Unsupported("visibility masks".into()));
        }
        let masks = self.visibility_masks.borrow();
        // No event for this view yet is the whole view being visible, which is the empty mask - not an
        // answer that claims the geometry is unknown.
        if let Some(Some(mask)) = masks.get(view) {
            out.vertices.clone_from(&mask.vertices);
            out.indices.clone_from(&mask.indices);
        }
        Ok(())
    }
}
