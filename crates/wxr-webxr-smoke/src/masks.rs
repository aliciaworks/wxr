//! What part of each view the browser says the display shows - the visibility mask, read back on its event.
//!
//! The event is the whole of the API: there is no query to make a mask appear, so a page learns the geometry
//! by being told it changed. The stub sends one for each view on the first animation frame, with a triangle
//! for the left eye and a quad for the right, so a cache indexed by the wrong view is caught by the numbers
//! rather than only by a count.

use super::*;

/// Check the mask a `visibilitymaskchange` named, and say what came through.
pub(super) fn check(session: &mut wxr_webxr::WebXrSession, view: usize) -> Result<(), String> {
    if !session.features().contains(wxr::Features::VISIBILITY_MASK) {
        return Err("a mask change arrived and the session did not claim the feature".into());
    }
    let (vertices, indices, first) = match view {
        0 => (3, 3, wxr::glam::Vec2::new(-0.9, -0.9)),
        1 => (4, 6, wxr::glam::Vec2::new(-0.8, -0.8)),
        _ => {
            return Err(format!(
                "a mask change named view {view}, and the stub has two"
            ));
        }
    };

    let mut mask = wxr::VisibilityMask::default();
    session
        .visibility_mask(view, &mut mask)
        .map_err(|error| format!("the mask for view {view} was refused: {error}"))?;
    if mask.is_whole_view() {
        return Err(format!("view {view}'s mask was read as the whole view"));
    }
    if mask.vertices.len() != vertices || mask.indices.len() != indices {
        return Err(format!(
            "view {view}'s mask came through as {} vertices and {} indices, not {vertices} and {indices}",
            mask.vertices.len(),
            mask.indices.len()
        ));
    }
    if (mask.vertices[0] - first).length() > 1e-6 {
        return Err(format!(
            "view {view}'s first vertex is {:?}, not {first:?}",
            mask.vertices[0]
        ));
    }

    say(&format!(
        "the browser showed view {view} through {vertices} vertices and {indices} indices"
    ));
    Ok(())
}
