//! The part of an eye's picture the display actually shows, which is WebXR's visibility mask.

use glam::Vec2;

/// The area of one eye's view the display can show, which is WebXR's `XRVisibilityMaskChangeEvent` read as
/// geometry.
///
/// A lens, a display's edge and a headset's nose gap mean that not every pixel of a rendered view reaches the
/// eye, and a renderer that knows which ones do can shade fewer fragments. The mask is a triangle mesh on the
/// plane one metre in front of the eye, before the view's projection turns it into clip space - so a vertex's
/// `z` is always `-1`, and what is carried here is its `x` and `y`.
///
/// An empty mask is not an empty view: it is the whole view being visible, which is what a runtime without a
/// mask has to say and what WebXR says for one it has not reported. [`VisibilityMask::is_whole_view`] is the
/// question to ask, rather than comparing lengths.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct VisibilityMask {
    /// The mesh's vertices, in the eye's view space on the plane `z = -1`, in metres.
    pub vertices: Vec<Vec2>,
    /// Indices into [`vertices`](Self::vertices), three per triangle, counter-clockwise as the user sees
    /// them. WebXR and OpenXR both promise a multiple of three.
    pub indices: Vec<u32>,
}

impl VisibilityMask {
    /// Whether the whole view is visible, which an empty mask says.
    pub fn is_whole_view(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }

    /// Forget the mask, which leaves the whole view visible - what a backend does before filling one in.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_geometry_is_the_whole_view() {
        assert!(VisibilityMask::default().is_whole_view());
        let mask = VisibilityMask {
            vertices: Vec::new(),
            indices: vec![0, 1, 2],
        };
        assert!(mask.is_whole_view());
    }

    #[test]
    fn triangles_are_not_the_whole_view() {
        let mask = VisibilityMask {
            vertices: vec![
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(0.0, 1.0),
            ],
            indices: vec![0, 1, 2],
        };
        assert!(!mask.is_whole_view());
    }

    #[test]
    fn clearing_leaves_the_whole_view_visible() {
        let mut mask = VisibilityMask {
            vertices: vec![Vec2::ZERO],
            indices: vec![0],
        };
        mask.clear();
        assert!(mask.is_whole_view());
    }
}
