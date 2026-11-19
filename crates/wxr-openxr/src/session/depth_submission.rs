//! The app's depth, which goes *to* the compositor: `XR_KHR_composition_layer_depth`.
//!
//! This is the extension whose direction is the ordinary one and whose neighbour's is not: [`super::depth`]
//! is the runtime measuring the room and handing the app a buffer to read, and this one is the app's own
//! depth buffer going out so the compositor can reproject the frame instead of guessing where the head will
//! be when the picture is shown. Both are called depth and neither is the other, which is why the core names
//! them apart: [`wxr_render::Import::target_depth`] is this one and [`wxr::Session::environment_depth`] is
//! the other.
//!
//! Three things have to hold before a frame's depth is submitted, and this module is all three:
//!
//! - **The instance was made with the extension.** That is `xrCreateInstance`'s answer, so it was decided
//!   before a session existed - see [`crate::OpenXr`].
//! - **A depth swapchain exists, in a format the renderer draws into.** `D32_SFLOAT` only: `wxr-render`
//!   builds one pipeline for [`wxr_render::DEPTH_FORMAT`] and falls back to a private buffer for anything
//!   else, so a swapchain in another format would be one nothing drew into - and submitting that is worse
//!   than submitting nothing, because the compositor would reproject a frame with an uninitialised buffer.
//! - **The app said what its depth values mean.** `XrCompositionLayerDepthInfoKHR` carries `nearZ`/`farZ`
//!   with no default, unlike WebXR's projection layer, so [`wxr::Session::set_depth_range`] is the answer and
//!   the switch at once: a session that was never told holds no depth back.
//!
//! The chain itself is the awkward part, and it is `openxr` 0.22's: it binds no
//! `CompositionLayerDepthInfoKHR`, so a projection view is built with the safe builder, `into_raw`ed, its
//! `next` pointed at a raw depth info, and `from_raw`ed back. The infos have to outlive the views that point
//! at them, which is why [`projection_views`] returns the two together and the caller keeps them until the
//! frame has been submitted.

use super::*;

/// The app's depth buffer, as a swapchain of its own.
///
/// A second swapchain rather than a second layer on the colour one: the compositor reads it to reproject and
/// the app writes it in the same pass, and OpenXR keeps what an app draws and what it hands over in separate
/// images. It is acquired and released in step with the colour image, because the two are one frame.
pub(super) struct DepthSwapchain {
    pub(super) swapchain: xr::Swapchain<xr::Vulkan>,
    /// Its images, as the runtime names them - a `VkImage`, which on this platform is an integer.
    pub(super) images: Vec<u64>,
    /// Which image this frame took, until it is given back.
    pub(super) held: Option<u32>,
}

impl DepthSwapchain {
    /// Make the depth swapchain, or `None` where the runtime will not give the renderer something to draw in.
    ///
    /// Failure is a session without depth submission and not a session that cannot be made: a compositor that
    /// reprojects is better than one that does not, and a headset that cannot is still a headset.
    pub(super) fn make(
        session: &xr::Session<xr::Vulkan>,
        formats: &[u32],
        extent: wxr::Extent2d,
    ) -> Option<Self> {
        // The one format worth asking for, and the reason is the renderer's rather than OpenXR's: `wgpu`
        // builds a pipeline for one depth format, and an attachment of another is a validation error. A
        // session whose depth swapchain were `D24_UNORM_S8_UINT` would draw into a private buffer and
        // submit an image the app never wrote - which is a wrong picture where no picture is an honest one.
        let format = ash::vk::Format::D32_SFLOAT.as_raw() as u32;
        if !formats.contains(&format) {
            log::info!(
                "wxr-openxr: no D32_SFLOAT swapchain format, so the app's depth stays private and unsubmitted"
            );
            return None;
        }
        let swapchain = match session.create_swapchain(&xr::SwapchainCreateInfo {
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            // Written by the app and read by the compositor, which is what a depth attachment is: no
            // `SAMPLED`, because nothing on this side samples it.
            usage_flags: xr::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            format,
            sample_count: 1,
            width: extent.width,
            height: extent.height,
            face_count: 1,
            // Two eyes, one array layer each - the same shape as the colour swapchain, because a projection
            // view names one layer of each and the two have to be the same eye.
            array_size: 2,
            mip_count: 1,
        }) {
            Ok(swapchain) => swapchain,
            Err(error) => {
                log::warn!("wxr-openxr: no depth swapchain: {error:?}");
                return None;
            }
        };
        let images = match swapchain.enumerate_images() {
            Ok(images) => images,
            Err(error) => {
                log::warn!("wxr-openxr: no depth images: {error:?}");
                return None;
            }
        };
        Some(Self {
            swapchain,
            images,
            held: None,
        })
    }

    /// This frame's depth image, once it has been taken.
    pub(super) fn image(&self) -> Option<u64> {
        self.held
            .and_then(|held| self.images.get(held as usize))
            .copied()
    }
}

impl OpenXrSession {
    /// Take this frame's depth image, when there is a depth swapchain to take one from.
    ///
    /// The wait is unbounded for the same reason the colour image's is: the runtime is the one that knows
    /// when the image is free, and a timeout would be this code deciding it knows better.
    pub(super) fn acquire_target_depth(&mut self) -> Result<(), wxr::Error> {
        let Some(depth) = self.target_depth.as_mut() else {
            return Ok(());
        };
        let held = depth
            .swapchain
            .acquire_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        depth
            .swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        depth.held = Some(held);
        Ok(())
    }

    /// Give this frame's depth image back, when one was taken.
    ///
    /// The binding refuses to release an image it never waited for, so the `held` check is the same rule the
    /// colour image follows after a failed wait: the runtime has already lost that image.
    pub(super) fn release_target_depth(&mut self) -> Result<(), wxr::Error> {
        let Some(depth) = self.target_depth.as_mut() else {
            return Ok(());
        };
        if depth.held.take().is_none() {
            return Ok(());
        }
        depth
            .swapchain
            .release_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))
    }

    /// Say which colour image and which depth image this frame is holding, for [`wxr::Session::image`].
    ///
    /// Both halves are the *acquired* images and not the swapchains' first: which one the runtime handed over
    /// changes every frame, and a renderer given the wrong one would draw into a picture the compositor is not
    /// reading. The two are picked independently, which is why the pair is made here rather than looked up by
    /// one position in two lists.
    pub(super) fn record_frame_images(&mut self) {
        self.frame_image = None;
        let Some(held) = self.held else {
            return;
        };
        // The runtime names an image it enumerated, so this is always a hit - and a miss is no picture rather
        // than image zero, which would be a texture of a picture nobody presents.
        let Some(color) = self.images.get(held as usize).copied() else {
            return;
        };
        let depth = self.target_depth.as_ref().and_then(DepthSwapchain::image);
        let motion = self
            .motion
            .as_ref()
            .and_then(super::space_warp::MotionSwapchain::image);
        self.frame_image = Some(crate::import::SwapchainImage {
            color,
            depth,
            motion,
        });
    }
}

/// The projection layer's views, one per eye, with the app's depth chained onto each when there is any.
///
/// The returned depth infos are what those views' `next` pointers point at, so they go back to the caller
/// rather than out of scope here: a chain that was dropped before the layer was submitted is a pointer into
/// freed memory inside `xrEndFrame`.
pub(super) fn projection_views<'a>(
    swapchain: &'a xr::Swapchain<xr::Vulkan>,
    located: &[xr::View],
    extent: xr::Extent2Di,
    depth: Option<(&DepthSwapchain, wxr::DepthRange)>,
    motion: Option<(
        &super::space_warp::MotionSwapchain,
        &DepthSwapchain,
        wxr::DepthRange,
    )>,
) -> (
    Vec<xr::CompositionLayerProjectionView<'a, xr::Vulkan>>,
    Vec<openxr_sys::CompositionLayerDepthInfoKHR>,
    Vec<openxr_sys::CompositionLayerSpaceWarpInfoFB>,
) {
    // Built to its full length before any view points at it: a `Vec` that grows moves its elements, and a
    // `next` taken before the last push would be a pointer to wherever that element used to be.
    let chain: Vec<openxr_sys::CompositionLayerDepthInfoKHR> = depth
        .map(|(depth, range)| {
            (0..located.len())
                .map(|index| depth_info(depth, index, extent, range))
                .collect()
        })
        .unwrap_or_default();
    // Built in full before the depth chain points at it, for the same reason the depth chain is built
    // before the views: a `Vec` that grows moves its elements, and a `next` is a raw pointer.
    let warp: Vec<openxr_sys::CompositionLayerSpaceWarpInfoFB> = motion
        .map(|(motion, depth, range)| {
            (0..located.len())
                .map(|index| super::space_warp::info(motion, depth, index, extent, range))
                .collect()
        })
        .unwrap_or_default();
    let mut chain = chain;
    for (index, info) in chain.iter_mut().enumerate() {
        if let Some(warp) = warp.get(index) {
            info.next = warp as *const _ as *const _;
        }
    }
    let views = located
        .iter()
        .enumerate()
        .map(|(index, view)| {
            let built = xr::CompositionLayerProjectionView::new()
                .pose(view.pose)
                .fov(view.fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(swapchain)
                        .image_array_index(index as u32)
                        .image_rect(xr::Rect2Di {
                            offset: xr::Offset2Di { x: 0, y: 0 },
                            extent,
                        }),
                );
            match chain.get(index) {
                // SAFETY: the depth info is in `chain`, which is returned beside these views and outlives
                // them, and nothing appends to it after this point - so the one reference taken here stays
                // valid for as long as the layer does.
                Some(info) => unsafe {
                    let mut raw = built.into_raw();
                    raw.next = info as *const _ as *const _;
                    xr::CompositionLayerProjectionView::from_raw(raw)
                },
                None => match warp.get(index) {
                    Some(info) => unsafe {
                        let mut raw = built.into_raw();
                        raw.next = info as *const _ as *const _;
                        xr::CompositionLayerProjectionView::from_raw(raw)
                    },
                    None => built,
                },
            }
        })
        .collect();
    (views, chain, warp)
}

/// One eye's line in that chain: which image its depth is in, and what the values in it mean.
fn depth_info(
    depth: &DepthSwapchain,
    index: usize,
    extent: xr::Extent2Di,
    range: wxr::DepthRange,
) -> openxr_sys::CompositionLayerDepthInfoKHR {
    openxr_sys::CompositionLayerDepthInfoKHR {
        ty: openxr_sys::StructureType::COMPOSITION_LAYER_DEPTH_INFO_KHR,
        next: std::ptr::null(),
        sub_image: openxr_sys::SwapchainSubImage {
            swapchain: depth.swapchain.as_raw(),
            image_rect: openxr_sys::Rect2Di {
                offset: openxr_sys::Offset2Di { x: 0, y: 0 },
                extent: openxr_sys::Extent2Di {
                    width: extent.width,
                    height: extent.height,
                },
            },
            // The eye's own array layer, and never zero for the right one: the two eyes are two layers of
            // one image, and a depth chain that named the left layer twice would reproject the right eye
            // with the left one's distances.
            image_array_index: index as u32,
        },
        // The ends of the buffer's own range, which for the depth state `wxr-render` makes is zero to one -
        // near things small, far things one, which is Vulkan's convention and the one OpenXR expects here.
        min_depth: 0.0,
        max_depth: 1.0,
        // What those numbers mean in metres, which is the app's projection and only the app knows it: the
        // compositor cannot recover a distance from a buffer without the planes it was projected between.
        near_z: range.near,
        far_z: range.far,
    }
}
