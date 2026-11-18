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
    type EnvironmentDepth = ();

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
        // Hit testing, which is neither provider: a ray is cast against whichever shape of the room came up,
        // so either one is enough and the bit says the cast can be made.
        if self.arkit.as_ref().is_some_and(ArKit::has_hit_test) {
            features = features.union(wxr::Features::HIT_TEST);
        }
        // Light estimation, which is ARKit's environment probe rather than either shape provider above: a
        // session can trace a room as triangles and still be unable to photograph it, and the bit is the
        // provider being up rather than a picture having arrived - a room too dark to photograph answers
        // with no intensity and zero harmonics, which the core says means nothing was measured.
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

    /// Whether the platform let this session have `feature`, which on this backend is ARKit's authorization
    /// for the provider that serves it - and the reason `features` can lose a bit after the query answers: a
    /// wearer who refused the room leaves a provider up that never reports anything.
    ///
    /// A capability no provider of this session serves answers `Granted`: there is no permission to be
    /// refused for something that is not here, and `features` is where an app reads that.
    fn authorization(&mut self, feature: wxr::Features) -> wxr::Authorization {
        self.arkit
            .as_ref()
            .map_or(wxr::Authorization::Granted, |arkit| {
                arkit.authorization(feature)
            })
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        // The press edges the last `inputs` produced come first: they are this crate's own, and waiting for
        // the layer to have another state to report would be a delay with no cause.
        if let Some(event) = self.controllers.poll() {
            return Some(event);
        }
        let (state, visibility) = self.layer_state();
        if state != self.reported.0 {
            self.reported.0 = state;
            return Some(wxr::Event::StateChanged(state));
        }
        // The layer being invalidated is the session being gone, and that is `Lost` once, after the state
        // that said it was over. A visibility read after that is not news: an ended session has no display
        // to be hidden from, and the other two backends stop before reading one.
        if state == wxr::State::Ended {
            if self.lost {
                return None;
            }
            self.lost = true;
            return Some(wxr::Event::Lost);
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
    /// The depth buffer this compositor hands over is only useful to it if it knows what the values mean,
    /// and it cannot read that off the picture - so the app says, in metres. The compositor uses it to
    /// reproject: when the prediction was off it moves the pixels to where the head turned out to be, and
    /// depth tells it how far each one is. This is the platform the core's depth range was written for.
    fn set_depth_range(&mut self, range: wxr::DepthRange) {
        self.depth_range = Some(range);
    }

    fn set_render_quality(&mut self, level: f32) {
        self.set_render_quality_impl(level);
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
        self.spaces.push((kind, wxr::Pose::IDENTITY));
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
        let Some((kind, inside)) = self.spaces.get(base.id() as usize).copied() else {
            return Err(wxr::Error::NoSpace(base.kind));
        };
        if kind != base.kind {
            return Err(wxr::Error::NoSpace(base.kind));
        }
        self.spaces.push((kind, inside.then(offset)));
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn locate(
        &mut self,
        space: wxr::ReferenceSpace,
        base: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::TrackedPose>, wxr::Error> {
        self.locate_impl(space, base)
    }

    /// Taking the compositor's next frame and reading its views: `frame` keeps it, and the ordering there is
    /// the compositor's requirement rather than this backend's policy.
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
    /// A source whose [`wxr::InputSource::hand`] was false has no skeleton, and asking anyway is an empty hand
    /// rather than an error - the core says to ask first, and a caller that did not can live with the answer.
    fn hand(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        self.hand_impl(source, space, out)
    }

    /// A controller's product category, which is the name GameController gives it. A hand is ARKit's and has
    /// no profile in this framework, so it answers with nothing; a pad is a `GCController`, whose
    /// `productCategory` is what an app switches on to draw the right picture.
    fn profiles(&mut self, source: wxr::InputId, out: &mut Vec<String>) -> Result<(), wxr::Error> {
        self.controllers.profiles(source, out);
        Ok(())
    }

    /// A buzz on a controller, which is a Core Haptics pattern played on an engine for it.
    ///
    /// A source with no actuator - every hand, and a pad GameController says has none - is `Unsupported`
    /// rather than a quiet nothing, which is the answer `InputSource::haptics` lets a caller avoid asking
    /// for. `play_pcm` is not here and never will be: Apple's haptics are patterns, not waveforms.
    fn pulse(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        self.controllers.pulse(source, intensity, duration)
    }

    /// The buzz taken back, which is the retained player's `stopAtTime:`.
    fn stop_pulse(&mut self, source: wxr::InputId) -> Result<(), wxr::Error> {
        self.controllers.stop(source)
    }

    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        // As `inputs`: the surfaces this frame, not the ones from every frame before it.
        out.clear();
        // The space before ARKit: a handle this session did not hand out is refused whether or not there is a
        // room to bring it to, and a session with no room still answers with no planes for its own space.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // ARKit reports planes in the session's own origin, which is this backend's `Local`; a caller asking
        // for another space gets them brought over, the same way the views are.
        let space_from_world = wxr_render::pose_from_transform(self.space_from_world(space));
        out.extend(arkit.planes().into_iter().map(|mut plane| {
            plane.pose = space_from_world.then(plane.pose);
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
        // The space first, for the reason `planes` gives.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // The same space the planes are brought to, and for the same reason: ARKit traces in the session's
        // own origin, which is this backend's `Local`, so a caller asking for another space gets the meshes
        // brought over rather than a refusal.
        let space_from_world = wxr_render::pose_from_transform(self.space_from_world(space));
        out.extend(arkit.meshes().into_iter().map(|mut mesh| {
            mesh.pose = space_from_world.then(mesh.pose);
            mesh
        }));
        Ok(())
    }

    /// A ray into the room, which ARKit does not cast itself - so this is the room's own geometry, read the
    /// way `meshes` and `planes` read it and asked where the ray meets it. See [`ArKit::cast`].
    ///
    /// A space this session did not hand out is refused first, then a session with none of the shapes asked
    /// for, and only then a ray that names no direction - the same order OpenXR's raycaster answers in.
    fn hit_test_source(
        &mut self,
        space: wxr::ReferenceSpace,
        kinds: wxr::HitTestKinds,
        ray: Option<wxr::Ray>,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        self.hit_test_source_impl(space, kinds, ray)
    }

    /// Where that ray lands this frame, in the space given - one hit, the nearest surface.
    ///
    /// WebXR's `getHitTestResults` is a list, and this backend has at most one thing to put in it: a ray meets
    /// the room once, at the nearest surface, unless the room has two surfaces in the same place.
    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        self.hits_impl(source, space, out)
    }

    /// Let a source go. The slot is emptied and `hits` on it answers an empty list, which is the core's shape
    /// for a released source; there is nothing else to tell ARKit, which never heard of it.
    fn release_hit_test_source(&mut self, source: wxr::HitTestSource) {
        self.hits.release(source);
    }

    /// A probe of the room's light, which is the provider being up rather than a promise of a picture.
    ///
    /// The one handle here that does not wait for anything: ARKit's provider is already running by the time
    /// a session exists, and what is asynchronous is the photograph rather than the probe. Asking for the
    /// estimate immediately after is not too early - it is the no-intensity one, which is the core's own
    /// word for a room it has not measured yet.
    fn light_probe(&mut self) -> Result<wxr::LightProbe, wxr::Error> {
        if self.arkit.as_ref().is_some_and(ArKit::has_light) {
            self.light_probe = true;
            return Ok(wxr::LightProbe::new(0));
        }
        Err(wxr::Error::Unsupported("light estimation".into()))
    }

    /// What the room's light is this frame, which is ARKit's last photograph as the core's harmonics.
    ///
    /// Every value if it has one and no intensity with zero harmonics if it does not, which is what the
    /// core says a runtime with no estimate hands over. A dark room and a room ARKit has not got round to
    /// yet are the same answer, and both are a scene lit by nothing rather than by a guess.
    ///
    /// A handle this session did not hand out gets that same no-estimate answer rather than the room's
    /// light: the handle names the probe, and this session never made that one. It is the answer WebXR gives
    /// an id outside its list, and an empty estimate rather than an error - the core's `light` has none.
    fn light(
        &mut self,
        probe: wxr::LightProbe,
        out: &mut wxr::LightEstimate,
    ) -> Result<(), wxr::Error> {
        let known = self.light_probe && probe.id() == 0;
        *out = self
            .arkit
            .as_ref()
            .filter(|_| known)
            .and_then(ArKit::light)
            .unwrap_or_default();
        Ok(())
    }

    fn anchor(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        // The space first, as every other entry that takes one does - and as the other backends now do.
        // A handle this session did not hand out is not one to build a world transform from, whether or not
        // anchors are available; answering `Unsupported` for it would make the answer depend on the feature.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        if self.arkit.is_none() {
            return Err(wxr::Error::Unsupported("anchors".into()));
        }
        // The anchor's place is asked for in `space`; ARKit takes one in the session's origin, which this
        // backend's `Local` is - so the space's own transform is composed in front.
        let world_from_anchor = self.world_from_space(space) * Mat4::from(pose.transform());
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
        // The space first, for the reason `planes` gives.
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        let Some(arkit) = self.arkit.as_mut() else {
            return Ok(None);
        };
        let Some(world) = arkit.anchor_pose(anchor) else {
            return Ok(None);
        };
        let space_from_world = wxr_render::pose_from_transform(self.space_from_world(space));
        Ok(Some(
            space_from_world.then(wxr_render::pose_from_transform(world)),
        ))
    }

    fn release_anchor(&mut self, anchor: wxr::Anchor) {
        if let Some(arkit) = self.arkit.as_mut() {
            arkit.release_anchor(anchor);
        }
    }

    fn anchor_id(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        self.arkit.as_mut()?.anchor_id(anchor)
    }

    /// A handle for a place ARKit kept under a name from an earlier run, rather than an `ARWorldMap`: a world
    /// anchor it has accepted persists across restarts until removed, and `copy_all_world_anchors` lists them.
    /// The handle arrives before the place does, like every anchor here: `anchor_pose` is `None` until the
    /// copy lands, and stays `None` for a name ARKit does not have.
    fn restore_anchor(&mut self, id: wxr::AnchorId) -> Result<wxr::Anchor, wxr::Error> {
        self.arkit
            .as_mut()
            .and_then(|arkit| arkit.restore_anchor(id))
            .ok_or_else(unsupported_restore)
    }

    fn persistent_anchor_ids(&mut self, out: &mut Vec<wxr::AnchorId>) -> Result<(), wxr::Error> {
        out.clear();
        let Some(arkit) = self.arkit.as_mut() else {
            return Err(unsupported_restore());
        };
        arkit
            .persistent_ids(out)
            .then_some(())
            .ok_or_else(unsupported_restore)
    }

    /// Forget a name for good, which ARKit takes by identifier.
    ///
    /// The request is made whether or not ARKit still has the name, and the core treats a name the runtime
    /// does not have as a no-op rather than an error.
    fn delete_persistent_anchor(&mut self, id: wxr::AnchorId) -> Result<(), wxr::Error> {
        let Some(arkit) = self.arkit.as_mut() else {
            return Err(unsupported_restore());
        };
        if arkit.delete_persistent_anchor(id) {
            Ok(())
        } else {
            Err(unsupported_restore())
        }
    }

    fn end(&mut self, frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.end_impl(frame)
    }
}

/// Why this session cannot answer a question about a name from an earlier run: ARKit keeps world anchors
/// between sessions, but only a world-tracking provider can list or forget them.
fn unsupported_restore() -> wxr::Error {
    wxr::Error::Unsupported(
        "a place kept from an earlier session: this session has no ARKit world tracking".into(),
    )
}
