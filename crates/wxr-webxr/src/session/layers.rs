//! The layers a browser session can hand over.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;
use crate::sys::{
    DomPointInit, XrCompositionLayer, XrCubeLayer, XrCylinderLayer, XrEquirectLayer, XrQuadLayer,
    XrgpuCubeLayerInit, XrgpuCylinderLayerInit, XrgpuEquirectLayerInit,
};

impl WebXrSession {
    pub(super) fn binding_impl(&mut self, layer: wxr::Layer) -> Result<wxr::Binding, wxr::Error> {
        // WebXR's binding is per GPU device, and a session has exactly one device and therefore one binding -
        // the one `start_gpu` made. So this hands that back rather than making another: a second would be a
        // second view of the same GPU device, which is not a thing the API has.
        if self.gpu.is_none() {
            return Err(wxr::Error::Unsupported(
                "the session has no GPU device to bind from".into(),
            ));
        }
        // The binding itself is session-wide, but it is still a binding *for a layer*: a caller holding a
        // handle this session did not hand out has nothing to bind, and saying so is the same answer
        // `set_layer_pose` gives for a layer that is gone.
        if self
            .layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(wxr::Error::Unsupported("no such layer".into()));
        }
        Ok(wxr::Binding::new(0))
    }
}

impl WebXrSession {
    pub(super) fn sub_image_impl(
        &mut self,
        binding: wxr::Binding,
        view: usize,
    ) -> Option<wxr::SubImage> {
        // There is one binding per session, so any other id is a handle from a session other than this one:
        // `None`, as the core promises, rather than a sub-image of whatever this session happens to have.
        if binding.id() != 0 {
            return None;
        }
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
        // Every shape here is a non-projection layer, and the specification makes them all part of the
        // `layers` feature: a session that was granted only `webgpu` has a projection layer and nothing else,
        // and a create call would be a `NotSupportedError` rather than a layer.
        let session = self.inner.borrow().session.clone();
        let Some(session) = session else {
            return Err(wxr::Error::Unavailable(
                "the session has not arrived yet".into(),
            ));
        };
        if !has_feature(&session, "layers") {
            return Err(wxr::Error::Unsupported("layers were not granted".into()));
        }
        // An equirect and a cube are environment layers, and the specification restricts them to a reference
        // space that is not the viewer: a skybox locked to the head defeats the reprojection it is for.
        if matches!(
            shape,
            wxr::LayerShape::Cube | wxr::LayerShape::Equirect { .. }
        ) && space.kind == wxr::SpaceKind::Viewer
        {
            return Err(wxr::Error::Rejected(
                "an environment layer cannot live in a viewer space".into(),
            ));
        }
        // A cube's six faces are faces of a cube, so the one texture holds squares: the specification refuses
        // an extent that is not, and a non-square request is a picture the compositor would stretch.
        if matches!(shape, wxr::LayerShape::Cube) && pixels.width != pixels.height {
            return Err(wxr::Error::Rejected(
                format!(
                    "a cube layer's faces are square, where {}x{} is not",
                    pixels.width, pixels.height
                )
                .into(),
            ));
        }
        // A layer is made *in* a space, so a space that is still a promise is a layer to ask for again - the
        // same answer `hit_test_source` gives, for the same reason. A handle this session never handed out is
        // not a promise, though, and is the `NoSpace` every other entry that takes one answers.
        let Some(reference) = self.resolved_space(space)? else {
            return Err(wxr::Error::Unavailable(
                "the space has not resolved yet".into(),
            ));
        };
        let preferred = gpu.binding.get_preferred_color_format();
        // Each shape has its own init dictionary, and each one is made with the same four required members.
        // The constructors take them in the IDL's order, and the setters below are by name so that a
        // transposed pair would still compile.
        let layer: XrCompositionLayer = match shape {
            wxr::LayerShape::Quad { width, height } => {
                let init =
                    XrgpuQuadLayerInit::new(&preferred, &reference, pixels.height, pixels.width);
                init.set_view_pixel_height(pixels.height);
                init.set_view_pixel_width(pixels.width);
                // One picture for both eyes, which is the thing that makes a quad worth handing to a
                // compositor at all - and the only layout this backend asks for, though it is also the default.
                init.set_layout(XrLayerLayout::Mono);
                init.set_width(width);
                init.set_height(height);
                throws::create_quad_layer(&gpu.binding, &init)
                    .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))?
                    .unchecked_into()
            }
            wxr::LayerShape::Cylinder {
                radius,
                central_angle,
                aspect,
            } => {
                let init = XrgpuCylinderLayerInit::new(
                    &preferred,
                    &reference,
                    pixels.height,
                    pixels.width,
                );
                init.set_view_pixel_height(pixels.height);
                init.set_view_pixel_width(pixels.width);
                init.set_layout(XrLayerLayout::Mono);
                init.set_radius(radius);
                init.set_central_angle(central_angle);
                init.set_aspect_ratio(aspect);
                throws::create_cylinder_layer(&gpu.binding, &init)
                    .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))?
                    .unchecked_into()
            }
            wxr::LayerShape::Equirect {
                radius,
                central_horizontal,
                upper_vertical,
                lower_vertical,
            } => {
                let init = XrgpuEquirectLayerInit::new(
                    &preferred,
                    &reference,
                    pixels.height,
                    pixels.width,
                );
                init.set_view_pixel_height(pixels.height);
                init.set_view_pixel_width(pixels.width);
                init.set_layout(XrLayerLayout::Mono);
                init.set_radius(radius);
                init.set_central_horizontal_angle(central_horizontal);
                init.set_upper_vertical_angle(upper_vertical);
                init.set_lower_vertical_angle(lower_vertical);
                throws::create_equirect_layer(&gpu.binding, &init)
                    .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))?
                    .unchecked_into()
            }
            wxr::LayerShape::Cube => {
                let init =
                    XrgpuCubeLayerInit::new(&preferred, &reference, pixels.height, pixels.width);
                init.set_view_pixel_height(pixels.height);
                init.set_view_pixel_width(pixels.width);
                // Six faces in one texture, which is a mono layout: a stereo cube would be twelve faces and
                // two pictures, and this backend draws the same picture for both eyes everywhere else.
                init.set_layout(XrLayerLayout::Mono);
                throws::create_cube_layer(&gpu.binding, &init)
                    .map_err(|error| wxr::Error::Rejected(format!("{error:?}").into()))?
                    .unchecked_into()
            }
        };
        // A layer is a picture *over* what is already there, so its alpha is part of the picture: a panel drawn
        // with a transparent background is transparent, which it is not if the compositor ignores the channel.
        // Opaque content says the same thing with every alpha at one.
        layer.set_blend_texture_source_alpha(true);
        let id = self.layers.len() as u32;
        // The compositor can lose a layer's picture - it says so by setting `needsRedraw` and firing `redraw` -
        // and the event arrives on the layer. The handler hands it on through a queue the poll drains, the same
        // way a reference-space reset is passed on, because a poll is where the app hears everything else.
        let redraw = self.redraw.clone();
        let handler = Closure::<dyn FnMut(Event)>::new(move |_| {
            redraw
                .borrow_mut()
                .push_back(wxr::Event::Redraw(wxr::Layer::new(id)));
        });
        let mut slot = crate::layers::Slot {
            layer,
            shape,
            pose: wxr::Pose::IDENTITY,
            on_redraw: None,
        };
        slot.listen_for_redraw(&handler);
        slot.on_redraw = Some(handler);
        self.layers.push(Some(slot));
        // The browser composites what the render state names, so a layer it has not been told about is a
        // texture nothing reads - which is not a layer the core hands back. The one way to get here is to make
        // one from inside a frame, where `updateRenderState` throws; it is taken back out rather than left as a
        // handle to something no compositor will ever place.
        if let Err(error) = self.present_layers() {
            if let Some(Some(slot)) = self.layers.pop() {
                slot.layer.destroy();
            }
            return Err(error);
        }
        log::info!(
            "wxr-webxr: a {} layer at {}x{} px",
            shape.name(),
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
        // A sub-image with no pixels in its viewport is no place to draw, the same answer a view with no
        // sub-image gets. Keeping it would hand back a `LayerImage` whose viewport is a pass wgpu refuses.
        if viewport.width == 0 || viewport.height == 0 {
            return None;
        }
        self.layer_images.resize_with(self.layers.len(), || None);
        self.layer_images[index] = Some(image);
        let image = self.layer_images[index].as_ref()?;
        Some((image, wxr::LayerImage { meta, viewport }))
    }
}

impl WebXrSession {
    /// Whether the compositor wants this layer drawn again, which is `XRCompositionLayer.needsRedraw`.
    ///
    /// Read from the browser each time rather than remembered: the compositor is the one that sets it, when it
    /// loses the picture it had, and the drawing that answers it is the app taking the layer's image. A handle
    /// this session never handed out is not a layer that needs drawing.
    pub(super) fn layer_needs_redraw_impl(&self, layer: wxr::Layer) -> bool {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .is_some_and(|slot| slot.layer.needs_redraw())
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
        // taking one here. A cube is the exception: it has an orientation and no transform, because the
        // compositor always draws it around the viewer - the specification says only the orientation of its
        // space is considered - so a pose's rotation is what there is to honour, and its position is not.
        match slot.shape {
            wxr::LayerShape::Cube => {
                let cube: &XrCubeLayer = slot.layer.unchecked_ref();
                let point = DomPointInit::new();
                point.set_x(pose.orientation.x as f64);
                point.set_y(pose.orientation.y as f64);
                point.set_z(pose.orientation.z as f64);
                point.set_w(pose.orientation.w as f64);
                let orientation: crate::sys::DomPointReadOnly = point.unchecked_into();
                cube.set_orientation(&orientation);
            }
            wxr::LayerShape::Quad { .. } => {
                let layer: &XrQuadLayer = slot.layer.unchecked_ref();
                layer.set_transform(&rigid(pose)?);
            }
            wxr::LayerShape::Cylinder { .. } => {
                let layer: &XrCylinderLayer = slot.layer.unchecked_ref();
                layer.set_transform(&rigid(pose)?);
            }
            wxr::LayerShape::Equirect { .. } => {
                let layer: &XrEquirectLayer = slot.layer.unchecked_ref();
                layer.set_transform(&rigid(pose)?);
            }
        }
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

impl WebXrSession {
    /// Put the foveation amount on the layer, when there is one - a session with no binding has no layer, and
    /// an amount it never asked for is the layer's own default and is left alone.
    pub(super) fn apply_foveation(&self) {
        let (Some(gpu), Some(foveation)) = (&self.gpu, self.foveation) else {
            return;
        };
        gpu.layer.set_fixed_foveation(Some(foveation));
    }
}

impl WebXrSession {
    /// Put the near and far planes on the browser's render state, if there is a session to put them on.
    ///
    /// Called again when a session arrives, which is what lets an app set them while the session is still
    /// `Connecting`: a render state belongs to the browser's session, and there is none until there is one.
    pub(super) fn apply_depth_range(&self) {
        let (Some(range), Some(session)) = (self.depth_range, self.inner.borrow().session.clone())
        else {
            return;
        };
        let state = XrRenderStateInit::new();
        state.set_depth_near(range.near as f64);
        state.set_depth_far(range.far as f64);
        if let Err(error) = throws::update_render_state(&session, &state) {
            log::warn!("wxr-webxr: the near and far planes were refused: {error:?}");
        }
    }
}
