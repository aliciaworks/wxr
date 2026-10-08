//! The layers a browser session can hand over.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn binding_impl(&mut self, _layer: wxr::Layer) -> Result<wxr::Binding, wxr::Error> {
        // WebXR's binding is per GPU device, and a session has exactly one device and therefore one binding -
        // the one `start_gpu` made. So this hands that back rather than making another: a second would be a
        // second view of the same GPU device, which is not a thing the API has.
        if self.gpu.is_some() {
            Ok(wxr::Binding::new(0))
        } else {
            Err(wxr::Error::Unsupported(
                "the session has no GPU device to bind from".into(),
            ))
        }
    }
}

impl WebXrSession {
    pub(super) fn sub_image_impl(
        &mut self,
        _binding: wxr::Binding,
        view: usize,
    ) -> Option<wxr::SubImage> {
        // The browser's own `XRView` for that index, kept by `views` for exactly this - `getViewSubImage` is
        // asked one of those, not an index into anything this core has.
        let xr_view = self.frame_views.get(view)?.clone();
        let gpu = self.gpu.as_ref()?;
        // A view the browser will not give a sub-image for is no sub-image, which is the same answer as a
        // session with no binding at all.
        let sub = throws::get_view_sub_image(&gpu.binding, &gpu.layer, &xr_view).ok()?;

        let color = image_meta(&sub.color_texture());
        let depth = sub.depth_stencil_texture();
        let depth = (!depth.is_null_or_undefined()).then(|| image_meta(&depth));
        let viewport = sub.viewport();

        Some(wxr::SubImage {
            color_size: wxr::glam::UVec2::new(color.extent.width, color.extent.height),
            depth_size: depth
                .map(|meta| wxr::glam::UVec2::new(meta.extent.width, meta.extent.height)),
            viewport: wxr::Viewport {
                x: viewport.x().max(0) as u32,
                y: viewport.y().max(0) as u32,
                width: viewport.width().max(0) as u32,
                height: viewport.height().max(0) as u32,
            },
            // Which slice of a texture array this eye is, for a stereo layer that carries two pictures in one
            // texture - which is what the view descriptor says and what `views` already reads.
            array_index: Some(base_array_layer(&sub.get_view_descriptor())),
        })
    }
}

impl WebXrSession {
    pub(super) fn layer_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        shape: wxr::LayerShape,
        pixels: wxr::Extent2d,
    ) -> Result<wxr::Layer, wxr::Error> {
        let Some(gpu) = self.gpu.as_ref() else {
            return Err(wxr::Error::Unsupported(
                "this session has no binding to make a layer from".into(),
            ));
        };
        // A quad is the one shape this backend makes. The bit is per shape on the platform too, so a session
        // with one shape and not another is the ordinary case rather than an odd one.
        let (width, height) = match shape {
            wxr::LayerShape::Quad { width, height } => (width, height),
            other => return Err(wxr::Error::Unsupported(format!("{} layers", other.name()))),
        };
        // A layer is made *in* a space, so a space that is still a promise is a layer to ask for again - the
        // same answer `hit_test_source` gives, for the same reason.
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Err(wxr::Error::Unavailable(
                "the space has not resolved yet".into(),
            ));
        };
        let init = XrgpuQuadLayerInit::new(
            &gpu.binding.get_preferred_color_format(),
            &reference,
            pixels.height,
            pixels.width,
        );
        // Set again by name, because the constructor takes them in the IDL's order and this is the one place a
        // transposed pair would still compile.
        init.set_view_pixel_height(pixels.height);
        init.set_view_pixel_width(pixels.width);
        // One picture for both eyes, which is the thing that makes a quad worth handing to a compositor at all -
        // and the only layout this backend asks for, though it is also the default.
        init.set_layout(XrLayerLayout::Mono);
        init.set_width(width);
        init.set_height(height);
        let layer = throws::create_quad_layer(&gpu.binding, &init)
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))?;
        // A layer is a picture *over* what is already there, so its alpha is part of the picture: a panel drawn
        // with a transparent background is transparent, which it is not if the compositor ignores the channel.
        // Opaque content says the same thing with every alpha at one.
        layer.set_blend_texture_source_alpha(true);
        let id = self.layers.len() as u32;
        self.layers.push(Some(crate::layers::Slot {
            layer,
            pose: wxr::Pose::IDENTITY,
        }));
        // The browser composites what the render state names, so a layer it has not been told about is a
        // texture nothing reads.
        if let Err(error) = self.present_layers() {
            log::warn!("wxr-webxr: the new layer was not presented: {error}");
        }
        log::info!(
            "wxr-webxr: a quad layer, {width}x{height} m at {}x{} px",
            pixels.width,
            pixels.height
        );
        Ok(wxr::Layer::new(id))
    }
}

impl WebXrSession {
    pub(super) fn layer_image_impl(
        &mut self,
        layer: wxr::Layer,
    ) -> Option<(&FrameImage, wxr::LayerImage)> {
        let index = layer.id() as usize;
        let (image, meta, viewport) = {
            let gpu = self.gpu.as_ref()?;
            let frame = self.current.clone()?;
            let slot = self.layers.get(index)?.as_ref()?;
            // One picture per layer per frame, and a second ask in the same frame gets the same texture - so
            // this is the frame's picture, and the frame is what takes it back.
            let sub = throws::get_sub_image(&gpu.binding, &slot.layer, &frame, XrEye::None).ok()?;
            let color = sub.color_texture();
            let depth = sub.depth_stencil_texture();
            let viewport = sub.viewport();
            (
                FrameImage {
                    color: color.clone(),
                    depth: (!depth.is_null_or_undefined()).then_some(depth),
                },
                image_meta(&color),
                wxr::Viewport {
                    x: viewport.x().max(0) as u32,
                    y: viewport.y().max(0) as u32,
                    width: viewport.width().max(0) as u32,
                    height: viewport.height().max(0) as u32,
                },
            )
        };
        self.layer_images.resize_with(self.layers.len(), || None);
        self.layer_images[index] = Some(image);
        let image = self.layer_images[index].as_ref()?;
        Some((image, wxr::LayerImage { meta, viewport }))
    }
}

impl WebXrSession {
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
        // Relative to the layer's own space, which is why the slot keeps the space it was made in rather than
        // taking one here.
        slot.layer.set_transform(&rigid(pose)?);
        slot.pose = pose;
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn release_layer_impl(&mut self, layer: wxr::Layer) {
        let Some(slot) = self.layers.get_mut(layer.id() as usize) else {
            return;
        };
        let Some(slot) = slot.take() else {
            return;
        };
        // Told explicitly rather than left to the collector: a layer the runtime is not told about is a picture
        // the compositor keeps presenting.
        slot.layer.destroy();
        if let Err(error) = self.present_layers() {
            log::warn!("wxr-webxr: the layer list was not updated: {error}");
        }
    }
}
