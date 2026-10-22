//! What this session does when the core asks it something: the `wxr::Session` implementation.
//!
//! It is one `impl` and therefore one file - Rust allows a type one implementation of a trait - and it is
//! separated from the type it implements because the two are read for different reasons: the type and its
//! own methods are what this backend *is*, and this is every answer it gives.

use super::*;

impl wxr::Session for AppleSession {
    type Image = FrameImage;
    /// No depth to hand over: a `CompositorServices` drawable has a depth buffer, but it is the one being
    /// *drawn into* rather than a measurement of the room.
    type Depth = ();

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.layer_state().0
    }

    fn visibility(&self) -> wxr::Visibility {
        self.layer_state().1
    }

    fn features(&self) -> wxr::Features {
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
        // Light estimation, which is ARKit's environment probe rather than either shape provider above: a
        // session can trace a room as triangles and still be unable to photograph it, and the bit is the
        // provider being up rather than a picture having arrived - a room too dark to photograph answers
        // with the all-zero estimate the core says means nothing was measured.
        if self.arkit.as_ref().is_some_and(ArKit::has_light) {
            features = features.union(wxr::Features::LIGHT_ESTIMATION);
        }
        // Hand tracking, which is the provider rather than the skeleton: a device whose hand provider came up
        // is a device that reports hands, and whether any given joint is tracked this frame is the question
        // `Session::hand` answers per joint. Asking for the skeleton of a hand ARKit has lost is an empty
        // hand, which is what the bit is a warning about rather than a promise against.
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

    fn poll(&mut self) -> Option<wxr::Event> {
        let (state, visibility) = self.layer_state();
        if state != self.reported.0 {
            self.reported.0 = state;
            return Some(wxr::Event::StateChanged(state));
        }
        if visibility != self.reported.1 {
            self.reported.1 = visibility;
            return Some(wxr::Event::VisibilityChanged(visibility));
        }
        if self.inputs_changed {
            self.inputs_changed = false;
            return Some(wxr::Event::InputsChanged);
        }
        None
    }

    fn blend(&self) -> wxr::Blend {
        self.blend
    }

    /// The near and far planes the scene draws with.
    ///
    /// The depth buffer this compositor hands over is only useful to it if it knows what the values in it mean,
    /// and it cannot read that off the picture - so the app says, in metres. The compositor uses it to
    /// reproject the frame: when the prediction was off it moves the pixels to where the head turned out to
    /// be, and depth is what tells it how far each of them is. This is the platform the core's depth range was
    /// written for, and the one where it is already put to work.
    fn set_depth_range(&mut self, near: f32, far: f32) {
        self.depth_range = Some((near, far));
    }

    fn images(&self) -> wxr::ImageMeta {
        // The texture is the truth about itself: the layer's configuration is what the app *asked* for, and
        // the compositor is free to hand back something else - so the format, the size and the layer count are
        // read off the thing that will actually be drawn into, when there is one.
        //
        // Before a frame there is only the configuration, and answering with its format rather than with
        // `Unknown` is what lets a caller build its renderer up front. The extent is a frame's to know and is
        // left at zero until then; nothing draws into it before a drawable says how big it is.
        let Some(image) = self.textures.first() else {
            return wxr::ImageMeta {
                format: self.configured,
                ..Default::default()
            };
        };
        let texture = &image.color;
        let (format, width, height, layers) = (
            texture.pixelFormat(),
            texture.width() as u32,
            texture.height() as u32,
            texture.arrayLength() as u32,
        );
        wxr::ImageMeta {
            format: color_format(format),
            extent: wxr::Extent2d::new(width, height),
            layers,
        }
    }

    fn image_count(&self) -> usize {
        // A compositor may dedicate a texture to each view or layer the views in one; the drawable's
        // texture count is the number either way, and a view's texture map is what says which is which.
        self.textures.len()
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        self.textures.get(index)
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        // Two spaces this platform has: the device's own, where each eye is relative to the wearer, and -
        // when ARKit came up - the origin it tracks, which is fixed where the session began and is exactly
        // `Local`. A floor is neither: ARKit's origin is not a plane, so a scene that asks to stand on one
        // is told no rather than put at eye height.
        match (kind, &self.arkit) {
            (wxr::SpaceKind::Viewer, _) => {}
            (wxr::SpaceKind::Local, Some(arkit)) if arkit.is_world_tracked() => {}
            _ => return Err(wxr::Error::NoSpace(kind)),
        }
        self.spaces.push(wxr::Pose::IDENTITY);
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn offset_space(
        &mut self,
        base: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(inside) = self.spaces.get(base.id() as usize).copied() else {
            return Err(wxr::Error::NoSpace(base.kind));
        };
        self.spaces.push(inside.then(offset));
        Ok(wxr::ReferenceSpace::new(
            base.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    /// Taking the compositor's next frame and reading its views, which is a sequence rather than an
    /// answer: `frame` keeps it, and the ordering there is the compositor's requirement rather than this
    /// backend's policy.
    fn begin(&mut self, now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.begin_impl(now, out)
    }

    fn views(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        self.views_impl(space, out)
    }

    fn inputs(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        self.inputs_impl(space, out)
    }

    /// A hand's skeleton, which is ARKit's joints at this frame's poses.
    ///
    /// A source whose [`wxr::InputSource::hand`] was false has no skeleton to answer with, and asking
    /// anyway is an empty hand rather than an error - the core says to ask the question first, and a caller
    /// that did not is a caller with an answer it can live with.
    fn hand(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        self.hand_impl(source, space, out)
    }

    /// A buzz on a controller, which is a Core Haptics pattern played on an engine for it.
    ///
    /// A source with no actuator - every hand, and a pad GameController says has none - is `Unsupported`
    /// rather than a quiet nothing, which is the answer `InputSource::haptics` lets a caller avoid asking
    /// for. `play_pcm` is not implemented on this platform and never will be: Apple's haptics are patterns
    /// rather than waveforms, so there is nothing for samples to become.
    fn pulse(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        self.controllers.pulse(source, intensity, duration)
    }

    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        // As `inputs`: the surfaces this frame, not the ones from every frame before it.
        out.clear();
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // ARKit reports planes in the session's own origin, which is this backend's `Local`; a caller
        // asking for another space gets them brought over, the same way the views are.
        let origin = self.space_origin(space);
        let (_, orientation, position) = origin.to_scale_rotation_translation();
        let origin = wxr::Pose {
            position,
            orientation,
        };
        out.extend(arkit.planes().into_iter().map(|mut plane| {
            plane.pose = plane.pose.relative_to(origin);
            plane
        }));
        Ok(())
    }

    fn meshes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Mesh>,
    ) -> Result<(), wxr::Error> {
        // And the same for the room's triangles: a mesh copied from this frame is the only one that is still
        // where ARKit says it is.
        out.clear();
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // The same space the planes are brought to, and for the same reason: ARKit traces in the session's
        // own origin, which is this backend's `Local`, so a caller asking for another space gets the meshes
        // brought over rather than a refusal.
        let origin = self.space_origin(space);
        let (_, orientation, position) = origin.to_scale_rotation_translation();
        let origin = wxr::Pose {
            position,
            orientation,
        };
        out.extend(arkit.meshes().into_iter().map(|mut mesh| {
            mesh.pose = mesh.pose.relative_to(origin);
            mesh
        }));
        Ok(())
    }

    /// A probe of the room's light, which is the provider being up rather than a promise of a picture.
    ///
    /// The one handle here that does not wait for anything: ARKit's provider is already running by the time
    /// a session exists, and what is asynchronous is the photograph rather than the probe. Asking for the
    /// estimate immediately after is not too early - it is the all-zero one, which is the core's own word
    /// for a room it has not measured yet.
    fn light_probe(&mut self) -> Result<wxr::LightProbe, wxr::Error> {
        if self.arkit.as_ref().is_some_and(ArKit::has_light) {
            return Ok(wxr::LightProbe::new(0));
        }
        Err(wxr::Error::Unsupported("light estimation".into()))
    }

    /// What the room's light is this frame, which is ARKit's last photograph as the core's harmonics.
    ///
    /// Every value if it has one and all zero if it does not, which is what the core says a runtime with no
    /// estimate hands over. A dark room and a room ARKit has not got round to yet are the same answer, and
    /// both are a scene lit by nothing rather than by a guess.
    fn light(
        &mut self,
        _probe: wxr::LightProbe,
        out: &mut wxr::LightEstimate,
    ) -> Result<(), wxr::Error> {
        *out = self
            .arkit
            .as_ref()
            .and_then(ArKit::light)
            .unwrap_or_default();
        Ok(())
    }

    fn anchor(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        // The anchor's place is asked for in `space`; ARKit takes one in the session's origin, which this
        // backend's `Local` is - so the space's origin is composed in front, the way a view is.
        let world_from_anchor = self.space_origin(space) * Mat4::from(pose.transform());
        self.arkit
            .as_mut()
            .and_then(|arkit| arkit.add_anchor(world_from_anchor))
            .ok_or_else(|| wxr::Error::Unsupported("anchors".into()))
    }

    fn anchor_pose(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        let Some(arkit) = &self.arkit else {
            return Ok(None);
        };
        let Some(world) = arkit.anchor_pose(anchor) else {
            return Ok(None);
        };
        let (_, orientation, position) = world.to_scale_rotation_translation();
        let origin = self.space_origin(space);
        let (_, origin_orientation, origin_position) = origin.to_scale_rotation_translation();
        Ok(Some(
            wxr::Pose {
                position,
                orientation,
            }
            .relative_to(wxr::Pose {
                position: origin_position,
                orientation: origin_orientation,
            }),
        ))
    }

    fn release_anchor(&mut self, anchor: wxr::Anchor) {
        if let Some(arkit) = self.arkit.as_mut() {
            arkit.release_anchor(anchor);
        }
    }

    fn end(&mut self, frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.end_impl(frame)
    }
}
