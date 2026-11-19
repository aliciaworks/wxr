//! Meta's application space warp: the half-rate frame the compositor synthesises from our motion.
//!
//! The app draws motion vectors and depth, the compositor makes the frame between two of ours. Three things
//! have to hold, and this module is all three:
//!
//! - **The instance was made with `XR_FB_space_warp`** - decided at `xrCreateInstance`, see [`crate::OpenXr`].
//! - **A motion-vector swapchain exists, in the format the renderer writes.** Two half-floats, one array
//!   layer per eye, and the usage bit the extension adds and the bindings do not -
//!   `XR_SWAPCHAIN_USAGE_MOTION_VECTOR_BIT_FB`, which is why it is written out rather than named.
//! - **The app asked for it and drew into it.** [`wxr::Session::set_space_warp`] is the ask; the image is
//!   drawn through [`wxr_render::Import::target_motion`]. Either alone would have the compositor synthesise
//!   from an image nobody wrote, which is worse than not synthesising at all.
//!
//! The info struct is chained onto the projection view after the depth info, because a synthesised frame
//! needs both: the depth says where each pixel is and the motion vector says where it went.

use openxr as xr;

/// The motion vectors, as the swapchain they go out through.
pub(super) struct MotionSwapchain {
    pub(super) swapchain: xr::Swapchain<xr::Vulkan>,
    /// Its images, as the runtime names them - a `VkImage`, which on this platform is an integer.
    pub(super) images: Vec<u64>,
    /// Which image this frame took, until it is given back.
    pub(super) held: Option<u32>,
}

impl MotionSwapchain {
    /// Make the motion swapchain, or `None` where the runtime will not give the renderer one.
    ///
    /// Failure is a session without space warp, not a session that cannot be made: a headset that draws
    /// every frame at its own rate is still a headset.
    pub(super) fn make(
        session: &xr::Session<xr::Vulkan>,
        formats: &[u32],
        extent: wxr::Extent2d,
    ) -> Option<Self> {
        // `VK_FORMAT_R16G16_SFLOAT`, which is the one format the extension takes for motion vectors: two
        // components are a screen-space delta and a third would be a depth this image is not.
        let format = ash::vk::Format::R16G16_SFLOAT.as_raw() as u32;
        if !formats.contains(&format) {
            log::info!(
                "wxr-openxr: no R16G16_SFLOAT swapchain format, so app space warp stays off"
            );
            return None;
        }
        // `XR_SWAPCHAIN_USAGE_MOTION_VECTOR_BIT_FB`, which `openxr-sys` 0.14 predates. The type is
        // `#[repr(transparent)]` over `u64`, and this is the one value in it the extension adds.
        let usage = unsafe { std::mem::transmute::<u64, xr::SwapchainUsageFlags>(1 << 7) };
        let swapchain = match session.create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: usage,
            format,
            sample_count: 1,
            width: extent.width,
            height: extent.height,
            face_count: 1,
            // One layer per eye, the same shape as the colour swapchain: a projection view names one layer of
            // each and the two have to be the same eye.
            array_size: 2,
            mip_count: 1,
        }) {
            Ok(swapchain) => swapchain,
            Err(error) => {
                log::warn!("wxr-openxr: no motion-vector swapchain: {error:?}");
                return None;
            }
        };
        let images = match swapchain.enumerate_images() {
            Ok(images) => images,
            Err(error) => {
                log::warn!("wxr-openxr: no motion-vector images: {error:?}");
                return None;
            }
        };
        Some(Self {
            swapchain,
            images,
            held: None,
        })
    }

    /// This frame's motion-vector image, once it has been taken.
    pub(super) fn image(&self) -> Option<u64> {
        self.held
            .and_then(|held| self.images.get(held as usize))
            .copied()
    }

    /// Take this frame's motion-vector image.
    pub(super) fn acquire(&mut self) -> Result<(), wxr::Error> {
        let held = self
            .swapchain
            .acquire_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        self.swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        self.held = Some(held);
        Ok(())
    }

    /// Give this frame's motion-vector image back, when one was taken.
    pub(super) fn release(&mut self) -> Result<(), wxr::Error> {
        if self.held.take().is_none() {
            return Ok(());
        }
        self.swapchain
            .release_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))
    }
}

/// One eye's line in the chain: where its motion vectors are, and what its depth means.
///
/// `app_space_delta_pose` is identity because the app reports the pose it drew with through the frames
/// themselves; the extension's convention is that this is the *extra* delta the app knows about and the
/// runtime does not, and we have none.
pub(super) fn info(
    motion: &MotionSwapchain,
    depth: &super::depth_submission::DepthSwapchain,
    index: usize,
    extent: xr::Extent2Di,
    range: wxr::DepthRange,
) -> openxr_sys::CompositionLayerSpaceWarpInfoFB {
    let rect = openxr_sys::Rect2Di {
        offset: openxr_sys::Offset2Di { x: 0, y: 0 },
        extent: openxr_sys::Extent2Di {
            width: extent.width,
            height: extent.height,
        },
    };
    openxr_sys::CompositionLayerSpaceWarpInfoFB {
        ty: openxr_sys::StructureType::COMPOSITION_LAYER_SPACE_WARP_INFO_FB,
        next: std::ptr::null(),
        // `XR_COMPOSITION_LAYER_SPACE_WARP_INFO_FLAG_NONE`.
        layer_flags: unsafe {
            std::mem::transmute::<u64, openxr_sys::CompositionLayerSpaceWarpInfoFlagsFB>(0)
        },
        motion_vector_sub_image: openxr_sys::SwapchainSubImage {
            swapchain: motion.swapchain.as_raw(),
            image_rect: rect,
            image_array_index: index as u32,
        },
        app_space_delta_pose: openxr_sys::Posef {
            orientation: openxr_sys::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            position: openxr_sys::Vector3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        depth_sub_image: openxr_sys::SwapchainSubImage {
            swapchain: depth.swapchain.as_raw(),
            image_rect: rect,
            image_array_index: index as u32,
        },
        // The buffer's own range, as `depth_submission` submits it.
        min_depth: 0.0,
        max_depth: 1.0,
        near_z: range.near,
        far_z: range.far,
    }
}

impl super::OpenXrSession {
    /// Take this frame's motion-vector image, when there is a swapchain to take one from.
    pub(super) fn acquire_target_motion(&mut self) -> Result<(), wxr::Error> {
        match self.motion.as_mut() {
            Some(motion) => motion.acquire(),
            None => Ok(()),
        }
    }

    /// Give this frame's motion-vector image back, when one was taken.
    pub(super) fn release_target_motion(&mut self) -> Result<(), wxr::Error> {
        match self.motion.as_mut() {
            Some(motion) => motion.release(),
            None => Ok(()),
        }
    }
}
