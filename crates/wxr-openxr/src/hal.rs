//! Reading the native handles out of wgpu.
//!
//! This is the only file in the workspace that names a Vulkan type, and it exists because OpenXR has no
//! wgpu backend. Its graphics bindings are written against Vulkan, D3D12 and OpenGL - there is no
//! `XR_KHR_wgpu` and there is not going to be one, because wgpu is not a platform API - so a session has to
//! be handed the handles of whatever the renderer actually made.
//!
//! The handles come from wgpu itself, through its HAL, which is the sanctioned way to reach what wgpu made.
//! The renderer still creates the device and still owns it; nothing here creates, keeps or destroys
//! anything, and the pointers do not outlive the call that passes them on. That is the whole difference
//! between this and owning a device: the runtime is *told*, and it does not get to decide.
//!
//! On Windows the same shape applies with D3D12 (`XR_KHR_d3d12_enable`), and the function below is where
//! that would go. A backend has one file like this, and it is the only platform-shaped code in it.

use std::ffi::c_void;

use ash::vk::Handle as _;

/// The handles an OpenXR Vulkan binding wants, as the runtime's own types.
///
/// Raw pointers, because that is what [`openxr_sys`] declares them as: the runtime does not care whose
/// device it is, only that it is a real one.
#[derive(Clone, Copy, Debug)]
pub struct Native {
    pub instance: *const c_void,
    pub physical_device: *const c_void,
    pub device: *const c_void,
    pub queue_family_index: u32,
    pub queue_index: u32,
}

/// The Vulkan objects behind a wgpu instance and device, if that is what they are.
///
/// `None` when wgpu is on another backend - a browser's WebGPU, or Metal on a Mac - and then it is a
/// backend for a different platform that is wanted, not a fallback here.
///
/// # Safety
///
/// The caller must not destroy the wgpu device while an OpenXR session made from these handles is alive,
/// and the handles must not be used after it is. The session holds no reference to the device, because
/// holding one would be this crate deciding how long the renderer's device lives.
pub unsafe fn vulkan(instance: &wgpu::Instance, device: &wgpu::Device) -> Option<Native> {
    let hal_instance = unsafe { instance.as_hal::<wgpu::hal::api::Vulkan>()? };
    // wgpu keeps the raw instance behind a shared handle, and the raw device behind a `Deref` - two
    // spellings of the same thing, both of them wgpu's.
    let hal_device = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }?;
    Some(Native {
        instance: hal_instance
            .shared_instance()
            .raw_instance()
            .handle()
            .as_raw() as *const c_void,
        physical_device: hal_device.raw_physical_device().as_raw() as *const c_void,
        device: hal_device.raw_device().handle().as_raw() as *const c_void,
        queue_family_index: hal_device.queue_family_index(),
        // wgpu makes one graphics queue, and a queue index is which one of a family it is.
        queue_index: 0,
    })
}
