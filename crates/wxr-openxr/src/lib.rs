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
//! **Depth is not submitted, and cannot be through this crate.** A runtime takes an app's depth through
//! `XR_KHR_composition_layer_depth`, whose `CompositionLayerDepthInfoKHR` is chained onto each projection view -
//! and `openxr` 0.22 binds neither that structure nor any way to chain one onto a layer: `openxr-sys` has the
//! raw struct, and the crate's builders do not reach it. Doing it means building the layer through the `sys`
//! layer and ending the frame through it, which is a piece of work of its own rather than a line here. What it
//! would buy is what it buys on the other two backends: a compositor that reprojects with depth instead of
//! guessing. It is also why `set_depth_range` is the core's default no-op here rather than an override that
//! stores the planes: there is nothing to submit them with, and a field nothing reads is a field that lies.
//!
//! **Surfaces are not detected either, and here it is a raw API rather than a missing one.**
//! `XR_EXT_plane_detection` is the extension that would do it, and the crate does load its function pointers -
//! `raw::PlaneDetectionEXT`, reachable through `instance.exts()` - but there is no typed wrapper around them:
//! the app drives the state machine itself (begin, poll until `DONE`, read into a buffer it owns, begin again)
//! over `openxr-sys` structs and raw pointers. That is a piece of work of its own, and an untested one, because
//! Monado has no plane detection to test it against - so `planes` is the core's default, a session with no
//! surfaces, until it is done. **Anchors are the same shape**: `XR_EXT_spatial_anchor` is loaded as raw
//! function pointers like the rest of the spatial extensions, so `anchor` is the core's `Unsupported` until
//! they are driven by hand.

#![cfg(not(target_family = "wasm"))]

mod convert;
mod hal;
mod import;
mod input;
mod session;

pub use import::Images;
pub use session::OpenXrSession;

/// The pose conversion `input` reaches by name, kept at the root so a module can say `crate::pose`.
pub(crate) use convert::pose;

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

        let system = instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| Error::runtime("find a headset", error))?;

        let views = instance
            .enumerate_view_configuration_views(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|error| Error::runtime("enumerate the views", error))?;
        if views.len() != 2 {
            return Err(Error::Unsupported(format!(
                "the runtime offers {} views; this backend draws two",
                views.len()
            )));
        }

        let blend = *instance
            .enumerate_environment_blend_modes(system, xr::ViewConfigurationType::PRIMARY_STEREO)
            .map_err(|error| Error::runtime("enumerate blend modes", error))?
            .first()
            .ok_or_else(|| Error::Unsupported("the runtime offers no blend mode".into()))?;

        Ok(Self {
            instance,
            system,
            views,
            blend,
            prefer_hdr: false,
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
                wxr::Error::Unavailable(format!("{action}: {message}"))
            }
            Error::Unsupported(what) => wxr::Error::Rejected(what),
        }
    }
}
