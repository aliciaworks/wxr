//! The app's own pictures: making a composition layer, drawing into it, placing it, and giving it back.
//!
//! One subject because on OpenXR a layer *is* its swapchain: the runtime reads the image the layer names at
//! `xrEndFrame`, so there is no separate binding to make and a bound layer's image is the layer's own. These
//! are `wxr::Session`'s answers, delegated from `mod.rs`, which is the session that owns the swapchains.

use super::*;

impl OpenXrSession {
    pub(super) fn layer_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
        options: wxr::LayerOptions,
    ) -> Result<wxr::Layer, wxr::Error> {
        // One extension per non-quad shape, enabled when the instance is made or not at all - so the refusal
        // is per shape and reads what the capability bits report.
        let enabled = match shape {
            wxr::LayerShape::Quad { .. } => true,
            wxr::LayerShape::Cylinder { .. } => self.layers_enabled.cylinder,
            wxr::LayerShape::Equirect { .. } => self.layers_enabled.equirect,
            wxr::LayerShape::Cube => self.layers_enabled.cube,
        };
        if !enabled {
            return Err(wxr::Error::Unsupported(
                format!(
                    "{} layers: this runtime does not have the extension",
                    shape.name()
                )
                .into(),
            ));
        }
        // The kind this session recorded for the id, not the public half: a translated cube is placed by making
        // a reference space of that kind. Both halves are checked first, so a changed kind is `NoSpace`.
        let Some(space_id) = self.space_index(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let space_kind = self.kinds[space_id];
        let swapchain = self
            .session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                // The format the session is drawing the world in, which is the one the runtime offered and the
                // one the renderer's pipeline is already built for.
                format: self.format,
                sample_count: 1,
                width: pixels.width,
                height: pixels.height,
                // A cube is six faces of one image; every other shape is one. The array is one for all of
                // them: a layer is one picture per eye, which is the reason to hand it to a compositor.
                face_count: match shape {
                    wxr::LayerShape::Cube => 6,
                    _ => 1,
                },
                array_size: 1,
                mip_count: 1,
            })
            .map_err(|error| Error::runtime("create a layer's swapchain", error))?;
        let images: Vec<_> = swapchain
            .enumerate_images()
            .map_err(|error| Error::runtime("enumerate a layer's images", error))?
            .into_iter()
            // A layer has no depth of its own: the app's depth is the projection layer's, because that is the
            // picture the compositor reprojects - the panels over it are already where they are.
            .map(|color| crate::import::SwapchainImage {
                color,
                depth: None,
                motion: None,
            })
            .collect();
        let id = self.layers.len() as u32;
        self.layers.push(Some(Layer {
            swapchain,
            images,
            space: space_id,
            space_kind,
            pose: xr::Posef::IDENTITY,
            placed: None,
            placed_at: None,
            shape,
            extent: pixels,
            held: None,
            options,
        }));
        log::info!(
            "wxr-openxr: a {} layer at {}x{} px",
            shape.name(),
            pixels.width,
            pixels.height
        );
        Ok(wxr::Layer::new(id))
    }

    pub(super) fn layer_image_impl(
        &mut self,
        layer: wxr::Layer,
    ) -> Option<(&crate::import::SwapchainImage, wxr::LayerImage)> {
        let slot = self.layers.get(layer.id() as usize)?.as_ref()?;
        let held = slot.held?;
        let image = slot.images.get(held as usize)?;
        // One image and the whole of it: a layer's swapchain is the layer's, so there is no atlas to take a
        // sub-rectangle out of - which is the difference between this and the projection layer's sub-image.
        Some((
            image,
            wxr::LayerImage {
                meta: wxr::ImageMeta {
                    format: self.color,
                    extent: slot.extent,
                    // A cube's swapchain is six faces, one per array layer. Every other shape is one picture.
                    layers: if matches!(slot.shape, wxr::LayerShape::Cube) {
                        6
                    } else {
                        1
                    },
                },
                viewport: wxr::Viewport {
                    x: 0,
                    y: 0,
                    width: slot.extent.width,
                    height: slot.extent.height,
                },
            },
        ))
    }

    pub(super) fn set_layer_pose_impl(
        &mut self,
        layer: wxr::Layer,
        pose: wxr::Pose,
    ) -> Result<(), wxr::Error> {
        let Some(slot) = self
            .layers
            .get_mut(layer.id() as usize)
            .and_then(Option::as_mut)
        else {
            return Err(wxr::Error::Unsupported("no such layer".into()));
        };
        // Kept rather than applied: a layer's pose is read at `xrEndFrame`, so placing one is not a call into
        // the runtime - it is where the next frame says it is. A cube is the exception: its composition struct
        // has an orientation and no position, so one is placed by making a reference space it then uses.
        if matches!(slot.shape, wxr::LayerShape::Cube) && pose.position != wxr::glam::Vec3::ZERO {
            if slot.placed.is_none() || slot.placed_at != Some(pose.position) {
                let Some(offset) = self.offsets.get(slot.space).copied() else {
                    return Err(wxr::Error::NoSpace(slot.space_kind));
                };
                let at = wxr::Pose {
                    position: pose.position,
                    orientation: wxr::glam::Quat::IDENTITY,
                };
                let space = self
                    .session
                    .create_reference_space(
                        reference_space(slot.space_kind),
                        posef(offset.then(at)),
                    )
                    .map_err(|error| Error::runtime("place a cube layer", error))?;
                slot.placed = Some(space);
                slot.placed_at = Some(pose.position);
            }
        } else {
            slot.placed = None;
            slot.placed_at = None;
        }
        slot.pose = posef(pose);
        Ok(())
    }

    pub(super) fn release_layer_impl(&mut self, layer: wxr::Layer) {
        // Dropping the swapchain is the whole of it: OpenXR has no `destroy`, and a layer no longer named is gone.
        if let Some(slot) = self.layers.get_mut(layer.id() as usize) {
            *slot = None;
        }
    }

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

/// The composition-layer flags one layer's options ask for.
///
/// One bit of this core is in `XrCompositionLayerFlags`: with `BLEND_TEXTURE_SOURCE_ALPHA` the compositor
/// reads the picture's alpha and blends it with what is behind, and without it every texel is opaque. The
/// other bits are a deprecated aberration correction and two alpha conventions this core does not ask for.
pub(super) fn flags(options: wxr::LayerOptions) -> xr::CompositionLayerFlags {
    if options.blend_texture_source_alpha {
        xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
    } else {
        xr::CompositionLayerFlags::EMPTY
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

    /// The alpha bit is the only flag a layer can ask for, and it is off unless the app asked for it: an
    /// opaque picture that names no bit lets the compositor skip reading a channel that is always one.
    #[test]
    fn an_opaque_layer_asks_for_no_flags() {
        assert_eq!(
            flags(wxr::LayerOptions::default()),
            xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
        );
        assert!(
            flags(wxr::LayerOptions {
                blend_texture_source_alpha: false,
            })
            .is_empty()
        );
    }

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
