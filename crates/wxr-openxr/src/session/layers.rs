//! What a bound layer's image is, which is the one part of layers OpenXR does not answer with the shape.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here. The rest of
//! the layer lifecycle is in `mod.rs`, beside the session that owns the swapchains.

use super::*;

impl OpenXrSession {
    pub(super) fn binding_impl(&mut self, layer: wxr::Layer) -> Result<wxr::Binding, wxr::Error> {
        // On OpenXR a composition layer *is* its swapchain - the runtime reads the image the layer names at
        // `xrEndFrame` - so there is nothing to make here: the binding is the layer's own, named by the handle
        // the layer already has. A handle this session did not hand out has no swapchain behind it, though,
        // and binding to one would be a binding to the projection image under a false name.
        if self
            .layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(wxr::Error::Unsupported("no such layer".into()));
        }
        Ok(wxr::Binding::new(layer.id()))
    }

    pub(super) fn sub_image_impl(
        &mut self,
        binding: wxr::Binding,
        view: usize,
    ) -> Option<wxr::SubImage> {
        // The swapchain the layer draws into, which is the layer's own: the projection layer's images are
        // what `image` names, and they are not a `Layer` this could have been asked to bind.
        let layer = self
            .layers
            .get(binding.id() as usize)
            .and_then(Option::as_ref)?;
        // A layer's image is the runtime's for one frame, like the eyes', and `held` is the image this frame
        // took. After `end` there is no slice to name, the same reason `layer_image` answers nothing then.
        layer.held?;
        // A view past the layer's last slice is not a face this texture has, and answering with it would
        // point a renderer at memory outside the array.
        if view >= slices(layer.shape) {
            return None;
        }
        let extent = layer.extent;
        Some(wxr::SubImage {
            color_size: wxr::glam::UVec2::new(extent.width, extent.height),
            // The session's swapchain is a colour attachment and sampled with no depth image of its own, so
            // there is no depth size to report rather than a zero one.
            depth_size: None,
            // Each slice is the whole picture, not a viewport of a packed pair - which is why a cube's
            // `ImageMeta::layers` is six - so a view owns all of its own slice.
            viewport: wxr::Viewport {
                x: 0,
                y: 0,
                width: extent.width,
                height: extent.height,
            },
            array_index: Some(view as u32),
        })
    }
}

/// How many array slices a layer of this shape has.
///
/// A cube is the only shape that is more than one picture: its six faces are its six slices. A quad, a
/// cylinder and an equirect are each one image, so a second view of one of them names no slice at all.
fn slices(shape: wxr::LayerShape) -> usize {
    match shape {
        wxr::LayerShape::Cube => 6,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The face count is the layer's, not the eyes': a cube always has six and every other shape one, so
    /// `sub_image` can refuse an index that would be outside the swapchain's array.
    #[test]
    fn a_cube_has_six_slices_and_the_rest_have_one() {
        assert_eq!(slices(wxr::LayerShape::Cube), 6);
        assert_eq!(
            slices(wxr::LayerShape::Quad {
                width: 1.0,
                height: 1.0,
            }),
            1
        );
        assert_eq!(
            slices(wxr::LayerShape::Cylinder {
                radius: 1.0,
                central_angle: 1.0,
                aspect: 1.0,
            }),
            1
        );
        assert_eq!(
            slices(wxr::LayerShape::Equirect {
                radius: 1.0,
                central_horizontal: 1.0,
                upper_vertical: 1.0,
                lower_vertical: 1.0,
            }),
            1
        );
    }
}
