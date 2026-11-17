//! An OpenXR backend for [`wxr`], rendering with wgpu.
//!
//! The whole of this crate is shaped by one decision the core made: **the renderer makes the device, and
//! the session is told about it.** OpenXR offers the other order - `XR_KHR_vulkan_enable2` has the runtime
//! create the `VkInstance` and `VkDevice` for the app to adopt - and that is not the one this is built on,
//! because a runtime that makes the device also gets to decide what the device can do.
//!
//! So a session is created from `XR_KHR_vulkan_enable`, which takes handles that already exist. They come
//! from the wgpu device the renderer was given, read out through wgpu's HAL (`hal`), which is the only
//! place in the workspace that names a Vulkan type. Nothing here creates a device, and nothing here
//! outlives one.
//!
//! The frames are the other half, and they are where the core's [`wxr::Session::Image`] pulls its weight: a
//! session hands out images the compositor will present and nothing else. The renderer wraps them as `wgpu`
//! textures; the core carries them and never looks inside.
//!
//! **Depth is submitted, when the app says what it means.** `XR_KHR_composition_layer_depth` is how an app's
//! own depth reaches the compositor: the app draws into a depth swapchain of its own, and each projection view
//! carries an `XrCompositionLayerDepthInfoKHR` naming that image and the planes it was projected between, so
//! the compositor reprojects the frame instead of guessing where the head will be when it is shown. The
//! renderer draws into whatever depth the image carries - [`wxr_render::Import::target_depth`] is how this
//! backend hands it over - and that is why [`wxr::Session::set_depth_range`] is real here: the chain has to say
//! what the values mean, and a session that was never told holds its depth back rather than sending numbers a
//! compositor cannot use.
//!
//! A session has a depth swapchain only where two things hold, and both are decided before it exists: the
//! instance was made with the extension, and the runtime offers `D32_SFLOAT`, the one format the renderer's
//! pipeline is built for. Anything else would be a buffer the app drew into privately, and submitting that is
//! worse than submitting nothing. The chain itself is raw - `openxr` 0.22 binds no `CompositionLayerDepthInfoKHR`
//! - and both halves are in `session/depth_submission.rs`.
//!
//! **The view is bigger than the part the optics show.** `XR_KHR_visibility_mask` answers with the triangle
//! mesh a display actually presents, per eye, and says when it moves - a stencil a renderer can test against,
//! so the fragments outside the lens are never shaded. It is the one extension here whose answer follows the
//! headset on the wearer's face rather than the room, so each view's mask is kept until the runtime's event
//! drops it. A runtime without it draws every pixel it renders, which is a whole view and not a failure.
//!
//! **Reading the room is a different extension, and it is driven by hand.** `XR_EXT_plane_detection`,
//! `XR_FB_spatial_entity`, `XR_META_environment_raycast` and `XR_META_environment_depth` are the four that
//! answer the core's `planes`, `anchor`, `hits` and `environment_depth`, and `openxr` 0.22 loads all four's
//! function
//! pointers and wraps none of them - so each of the four is a state machine this crate drives over
//! `openxr-sys` structs and raw pointers, in a module of its own under `session/`. Two of them need care that
//! is not visible from the call site: `XR_META_environment_raycast` cannot be enabled without `XR_EXT_future`
//! or `xrCreateInstance` refuses the pair, and the depth map can only be taken once between a frame's begin
//! and its end.
//!
//! **An anchor's name is three extensions, not one.** `XR_FB_spatial_entity` makes the anchor and says
//! whether it can be stored (`STORABLE`), and Meta's `XR_META_spatial_entity_persistence` and
//! `XR_META_spatial_entity_discovery` are what actually keep it and find it again - so `anchor_id`,
//! `restore_anchor` and `delete_persistent_anchor` are real only where all three are. The pair is also where
//! the FB storage and query extensions are deliberately not used: the registry marks both obsolete, and the
//! META ones erase by UUID rather than by space. `session/persistence.rs` is that story.
//!
//! The honest part of that list is what a runtime that does not have one of them gets. Every extension is
//! asked for from a runtime that lists it and no others, and a session without one answers with the core's
//! `Unsupported`, an empty list, or `None` - so `features()` is the same fact read from the other side, and a
//! bit that says yes where the method says no would be a promise this backend is not keeping.
//!
//! What is *not* on that list is light estimation. There is no multivendor extension for it: the registry's
//! two are ByteDance's `XR_BD_spatial_light_estimation` and Android's `XR_ANDROID_light_estimation`, and
//! neither is in `openxr-sys` 0.14, which is what this crate drives the raw calls through. A runtime that
//! measures the room's light therefore has nothing to tell this crate with, so `light` and `light_probe` are
//! the core's defaults here for want of an extension this backend could ask for rather than want of the work.
//!
//! **Meshes are the one left undone**, and it is a choice rather than a gap: `XR_FB_triangle_mesh`,
//! `XR_META_spatial_entity_mesh`, `XR_BD_spatial_mesh` and `XR_ML_world_mesh_detection` are all bound, all
//! raw, and they are four different shapes - a triangle mesh with its own lifecycle, a component of a spatial
//! entity, and two whole-room scans. Picking one is picking which headsets this backend can mesh, and it has
//! not been picked.

// Not wasm, and not Apple: this backend speaks Vulkan, and `wgpu::hal::api::Vulkan` is a type that does
// not exist where wgpu has no Vulkan - so a macOS or visionOS build is a build where this crate is
// empty rather than one that fails on a type it cannot name.
#![cfg(all(not(target_family = "wasm"), not(target_vendor = "apple")))]

mod convert;
mod hal;
mod import;
mod input;
mod session;

pub use import::{Images, SwapchainImage};
pub use session::{EnvironmentDepthImage, OpenXrSession};
pub(crate) use session::{HapticExtensions, InputExtensions, LayerExtensions, SpatialExtensions};

/// The pose and velocity conversions `input` reaches by name, kept at the root so a module can say
/// `crate::pose` and `crate::motion`.
pub(crate) use convert::{motion, pose};

use openxr as xr;

/// What the renderer hands over: the wgpu objects it made, and owns.
#[derive(Clone, Debug)]
pub struct Device {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
}

/// An OpenXR runtime, connected to but not yet in a session.
pub struct OpenXr {
    instance: xr::Instance,
    system: xr::SystemId,
    views: Vec<xr::ViewConfigurationView>,
    blend: xr::EnvironmentBlendMode,
    /// Whether to ask for an HDR swapchain when a runtime offers one. Off by default - see [`OpenXr::prefer_hdr`].
    prefer_hdr: bool,
    /// Whether the runtime takes a display refresh rate. Fixed at `xrCreateInstance`, like the layer
    /// extensions, and the capability bit and the two calls read from here.
    refresh_rate: bool,
    /// Which composition-layer extensions the instance was made with. They are fixed at `xrCreateInstance`,
    /// so what a session can hand over was decided before the session existed - and both the capability bits
    /// and the per-shape refusal read from here rather than from the runtime.
    layers: LayerExtensions,
    /// Whether the instance was made with `XR_KHR_composition_layer_depth`, which is how the app's own depth
    /// reaches the compositor - the direction [`SpatialExtensions::depth`] is not. Fixed at the same moment,
    /// so a session that will submit no depth says so before it exists.
    depth_layer: bool,
    /// Which haptic extensions it was made with, fixed at the same moment and read by the capability bit and
    /// by the two output calls.
    haptics: HapticExtensions,
    /// Which of the world-modelling extensions it was made with, fixed at the same moment. What a session
    /// can say about the room was therefore decided before the session existed, which is the reason this is
    /// carried rather than asked again.
    spatial: SpatialExtensions,
    /// Which input extensions it was made with, fixed at the same moment: the profiles for hands and for the
    /// eyes, and the controllers whose own extension names them. A profile is a path the runtime only knows
    /// while its extension is enabled, so which of them can be suggested was decided here as well.
    input: InputExtensions,
    /// Whether the instance was made with `XR_KHR_visibility_mask`, which is how a runtime says which part of
    /// each eye's view is worth drawing. Fixed at `xrCreateInstance`, like the rest, and read by the capability
    /// bit and the one call.
    visibility_mask: bool,
}

impl OpenXr {
    /// Load the runtime this machine has and take the first headset it offers.
    ///
    /// An `Entry` is the loader rather than a runtime: which runtime it is belongs to the machine, and a
    /// machine with none is [`wxr::Error::Unavailable`] rather than a panic.
    pub fn load() -> Result<Self, Error> {
        // SAFETY: the loader the dynamic loader finds must be an OpenXR one, which is what the runtime
        // the machine has configured is. On platforms with a different loader story this is where that
        // would be said; `()` is the "ask the system" answer everywhere but Android.
        let entry =
            unsafe { xr::Entry::load(&()) }.map_err(|error| Error::runtime("load", error))?;

        // The *legacy* binding, and only it, because it is the one that takes an instance and a device that
        // already exist. Asking for both is not harmless: the crate prefers `XR_KHR_vulkan_enable2` when it
        // is there, and that is the path where the runtime makes the instance itself - the trap this backend
        // is written against. It is also the path where a runtime wants a `vkGetInstanceProcAddr` the app
        // never handed it, and says so: Monado refuses with
        // `xrGetVulkanGraphicsDeviceKHR(sys->vk_get_instance_proc_addr == NULL)`.
        let mut extensions = xr::ExtensionSet::default();
        extensions.khr_vulkan_enable = true;
        // A floor space is worth having and not worth failing over, so it is asked for only from a runtime
        // that lists it: `xrCreateInstance` refuses an extension the runtime does not have, and a session
        // that cannot be created at all is worse than one without a floor.
        let supported = entry
            .enumerate_extensions()
            .map_err(|error| Error::runtime("ask what the runtime supports", error))?;
        extensions.ext_local_floor = supported.ext_local_floor;
        // Hand tracking is asked for the same way: a session with no skeleton is a session, and an instance the
        // runtime refuses is not.
        extensions.ext_hand_tracking = supported.ext_hand_tracking;
        // The two hand-interaction profiles, which are a second story about hands: tracking is where the
        // fingers are, and these are how a pinch and a grasp become a button. Each profile belongs to its own
        // extension and is only a path the runtime knows while that extension is enabled, so the two are
        // asked for separately and remembered for the profile table.
        extensions.ext_hand_interaction = supported.ext_hand_interaction;
        extensions.msft_hand_interaction = supported.msft_hand_interaction;
        // The five controllers whose profile comes from an extension of its own. Same rule again - each only
        // from a runtime that lists it - and the table below is what reads the answer.
        extensions.ext_samsung_odyssey_controller = supported.ext_samsung_odyssey_controller;
        extensions.ext_hp_mixed_reality_controller = supported.ext_hp_mixed_reality_controller;
        extensions.htc_vive_cosmos_controller_interaction =
            supported.htc_vive_cosmos_controller_interaction;
        extensions.meta_touch_controller_plus = supported.meta_touch_controller_plus;
        extensions.fb_touch_controller_pro = supported.fb_touch_controller_pro;
        // The eyes' pose, which is a profile of its own rather than a hand - and the same rule once more.
        extensions.ext_eye_gaze_interaction = supported.ext_eye_gaze_interaction;
        // Foveation is two extensions that go together: one makes a profile, the other puts it on the swapchain
        // the compositor samples. Both only from a runtime that lists them, like the two above.
        extensions.fb_foveation = supported.fb_foveation;
        extensions.fb_swapchain_update_state = supported.fb_swapchain_update_state;

        // The three composition-layer shapes beyond the core quad, asked for the same way: each only from a
        // runtime that lists it. What the instance was made with is what a session can hand over, so it is
        // remembered rather than asked again - `xrCreateInstance` is the one place the answer exists.
        extensions.khr_composition_layer_cylinder = supported.khr_composition_layer_cylinder;
        extensions.khr_composition_layer_equirect2 = supported.khr_composition_layer_equirect2;
        extensions.khr_composition_layer_cube = supported.khr_composition_layer_cube;
        // And the one that is not a shape: the app's own depth, which a compositor reprojects with. The same
        // rule again, and whether the session has a swapchain to give it is decided in
        // `session/depth_submission.rs`.
        extensions.khr_composition_layer_depth = supported.khr_composition_layer_depth;
        // The display's rate: Meta's extension, and asked for the same way - only from a runtime that lists
        // it. A runtime without it is a runtime whose display runs at one rate, which is a fact and not a
        // failure.
        extensions.fb_display_refresh_rate = supported.fb_display_refresh_rate;
        // Which part of each eye's view the display shows. Asked for the same way - only from a runtime that
        // lists it - and a runtime without it draws every pixel it renders, which is a whole view and not a
        // failure.
        extensions.khr_visibility_mask = supported.khr_visibility_mask;

        // The four ways a runtime can say what the room is: the surfaces it has found, the places it is
        // keeping fixed, where a ray meets things, and how far away the walls are. Asked for the same way as
        // every extension above - only from a runtime that lists it - and an instance the runtime refuses is
        // worse than a session without a floor.
        //
        // One of them is two, in a way the name does not show: `XR_META_environment_raycast` requires
        // `XR_EXT_future` to be enabled alongside it or `xrCreateInstance` refuses the pair outright with
        // `XR_ERROR_EXTENSION_DEPENDENCY_NOT_ENABLED`. So they are decided together, and a runtime missing
        // the future is a runtime without a raycaster too - which is what `spatial` below will say.
        extensions.ext_plane_detection = supported.ext_plane_detection;
        extensions.fb_spatial_entity = supported.fb_spatial_entity;
        // The pair that keeps an anchor's name between runs: Meta's persistence for the save and erase, and
        // its discovery for finding a saved place again. Both, because a name that can be written and not
        // read back is not one - `SpatialExtensions::persistence` is where the pair is decided and
        // `session/persistence.rs` is what uses it. Both also depend on `XR_FB_spatial_entity`, which is why
        // they are gated on it: a runtime that lists one without the other would otherwise get an instance
        // that `xrCreateInstance` refuses for a missing dependency.
        extensions.meta_spatial_entity_persistence =
            supported.meta_spatial_entity_persistence && supported.fb_spatial_entity;
        extensions.meta_spatial_entity_discovery =
            supported.meta_spatial_entity_discovery && supported.fb_spatial_entity;
        extensions.meta_environment_depth = supported.meta_environment_depth;
        extensions.meta_environment_raycast =
            supported.meta_environment_raycast && supported.ext_future;
        extensions.ext_future = extensions.meta_environment_raycast;

        // Haptics. `XR_EXT_haptic_feedback` is the one that matters, and it is the one extension the crate
        // leaves out of its `ExtensionSet`: it knows the calls - `xrApplyHapticFeedback` is loaded with the
        // instance like every other entry point - but the set has no field naming it, so it is asked for by
        // name through the set's catch-all, and only from a runtime that lists it. Meta's waveform builds on
        // top of it, so it is asked for only where both are there.
        let feedback = supported
            .other
            .iter()
            .any(|name| name.as_slice() == b"XR_EXT_haptic_feedback".as_slice());
        if feedback {
            extensions.other.push(b"XR_EXT_haptic_feedback".to_vec());
        }
        extensions.fb_haptic_pcm = feedback && supported.fb_haptic_pcm;
        let haptics = HapticExtensions {
            feedback,
            pcm: extensions.fb_haptic_pcm,
        };
        let layers = LayerExtensions {
            cylinder: extensions.khr_composition_layer_cylinder,
            equirect: extensions.khr_composition_layer_equirect2,
            cube: extensions.khr_composition_layer_cube,
        };
        let spatial = SpatialExtensions {
            planes: extensions.ext_plane_detection,
            anchors: extensions.fb_spatial_entity,
            persistence: extensions.meta_spatial_entity_persistence
                && extensions.meta_spatial_entity_discovery,
            raycast: extensions.meta_environment_raycast,
            depth: extensions.meta_environment_depth,
        };
        // The loader validates this: an application with no name is not an application it will make an
        // instance for, and that is a real check rather than a formality - a runtime's logs are read by
        // whoever has to find out why the headset is not working.
        let app_info = xr::ApplicationInfo {
            application_name: "wxr",
            application_version: 1,
            engine_name: "wxr",
            engine_version: 1,
            ..Default::default()
        };
        let instance = entry
            .create_instance(&app_info, &extensions, &[], &())
            .map_err(|error| Error::runtime("create the instance", error))?;

        // Which runtime answered, before anything is asked of it: the name and version are the first two things
        // a runtime's own bug report wants, and reading them costs one call at startup.
        if let Ok(properties) = instance.properties() {
            log::info!(
                "wxr-openxr: runtime {} {:?}",
                properties.runtime_name,
                properties.runtime_version
            );
        }

        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| Error::runtime("find a headset", error))?;

        // The configuration this backend draws, asked for rather than assumed: the specification has an app
        // enumerate what the runtime offers and choose one, and a runtime that offers no stereo pair is one
        // this backend has nothing to draw into.
        let configuration = xr::ViewConfigurationType::PRIMARY_STEREO;
        let configurations = instance
            .enumerate_view_configurations(system)
            .map_err(|error| Error::runtime("enumerate view configurations", error))?;
        if !configurations.contains(&configuration) {
            return Err(Error::Unsupported(format!(
                "the runtime offers {} view configuration(s) and not primary stereo",
                configurations.len()
            )));
        }
        // And what it says about that one. A mutable field of view is the runtime telling an app it may choose
        // one; this backend hands back the runtime's own views, so the answer goes in the log rather than into
        // a decision.
        match instance.view_configuration_properties(system, configuration) {
            Ok(properties) => log::debug!(
                "wxr-openxr: primary stereo, field of view {}",
                if properties.fov_mutable {
                    "mutable"
                } else {
                    "fixed"
                }
            ),
            Err(error) => log::debug!("wxr-openxr: no view configuration properties: {error:?}"),
        }

        let views = instance
            .enumerate_view_configuration_views(system, configuration)
            .map_err(|error| Error::runtime("enumerate the views", error))?;
        if views.len() != 2 {
            return Err(Error::Unsupported(format!(
                "the runtime offers {} views; this backend draws two",
                views.len()
            )));
        }

        let blend = *instance
            .enumerate_environment_blend_modes(system, configuration)
            .map_err(|error| Error::runtime("enumerate blend modes", error))?
            .first()
            .ok_or_else(|| Error::Unsupported("the runtime offers no blend mode".into()))?;

        // The input flags are built here rather than with the other three because one of them is not a
        // question about the instance at all: the eyes are the system's, so whether there are any to follow
        // is asked of the system, which exists only now.
        let input = InputExtensions {
            ext_hand: extensions.ext_hand_interaction,
            msft_hand: extensions.msft_hand_interaction,
            samsung_odyssey: extensions.ext_samsung_odyssey_controller,
            hp_mixed_reality: extensions.ext_hp_mixed_reality_controller,
            vive_cosmos: extensions.htc_vive_cosmos_controller_interaction,
            touch_plus: extensions.meta_touch_controller_plus,
            touch_pro: extensions.fb_touch_controller_pro,
            // The extension says the runtime can do this; the system says whether there are eyes to
            // follow, and a source that can never be tracked is not a source.
            eye_gaze: extensions.ext_eye_gaze_interaction
                && input::eye_gaze_supported(&instance, system),
        };

        Ok(Self {
            instance,
            system,
            views,
            blend,
            prefer_hdr: false,
            layers,
            depth_layer: extensions.khr_composition_layer_depth,
            refresh_rate: extensions.fb_display_refresh_rate,
            haptics,
            spatial,
            input,
            visibility_mask: extensions.khr_visibility_mask,
        })
    }

    /// The Vulkan API version the runtime needs, as `(minimum, maximum)`.
    ///
    /// This has to be asked *before* a device is made: one below the minimum is a session that cannot be
    /// created, and the maximum is what the runtime can be handed. It is the first thing a renderer does,
    /// and the reason `load` and `connect` are two calls rather than one.
    ///
    /// A session cannot be created until it has been asked, so `connect` asks it too, for an app that did not.
    /// The answer is the same either way, and only an app can *act* on it - which is what this call is for.
    pub fn requirements(&self) -> Result<(xr::Version, xr::Version), Error> {
        let requirements = self
            .instance
            .graphics_requirements::<xr::Vulkan>(self.system)
            .map_err(|error| Error::runtime("ask the graphics requirements", error))?;
        Ok((
            requirements.min_api_version_supported,
            requirements.max_api_version_supported,
        ))
    }

    /// Ask for an HDR swapchain when the runtime offers one.
    ///
    /// Off by default, and it has to be asked for: an HDR image is drawn with values above one, so a scene that
    /// is not built for it clips where it would have looked brighter. Turning this on is half of that decision
    /// and the tone map is the other half, which is the app's - which is why it is a choice and not a default.
    ///
    /// It changes the format `images` reports, and a renderer builds its pipeline from that.
    pub fn prefer_hdr(&mut self, prefer: bool) {
        self.prefer_hdr = prefer;
    }

    /// The size the runtime recommends for each eye, which is what a swapchain is made at.
    pub fn recommended_extent(&self) -> wxr::Extent2d {
        wxr::Extent2d::new(
            self.views[0].recommended_image_rect_width,
            self.views[0].recommended_image_rect_height,
        )
    }
}

impl wxr::Backend for OpenXr {
    type Device = Device;
    type Session = OpenXrSession;

    /// OpenXR has no session mode to ask for. The system's configuration decides the form factor and the
    /// runtime decides how the picture blends, and `blend` is what reports what it chose - so the mode is taken
    /// and ignored, which is the honest answer for a platform that decides this itself.
    fn connect(
        &self,
        device: Device,
        _mode: wxr::SessionMode,
    ) -> Result<OpenXrSession, wxr::Error> {
        OpenXrSession::new(self, &device).map_err(wxr::Error::from)
    }
}

/// What went wrong before a session existed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the OpenXR runtime could not {action}: {message}")]
    Runtime { action: String, message: String },
    #[error("{0}")]
    Unsupported(String),
}

impl Error {
    fn runtime(action: &str, error: impl std::fmt::Debug) -> Self {
        Self::Runtime {
            action: action.to_string(),
            message: format!("{error:?}"),
        }
    }
}

impl From<Error> for wxr::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::Runtime { action, message } => {
                wxr::Error::Unavailable(format!("{action}: {message}").into())
            }
            Error::Unsupported(what) => wxr::Error::Rejected(what.into()),
        }
    }
}
