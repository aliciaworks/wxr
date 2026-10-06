//! Platform values in the core's terms.
//!
//! Everything here is one direction or the other between what OpenXR reports and what the core says, kept
//! together so the mappings can be read next to each other.

use openxr as xr;

use wxr::glam::{Quat, Vec3};

/// What a Vulkan swapchain format is in the core's terms.
///
/// This is the one place the three backends differ on colour: WebXR and a compositor platform hand a format
/// over, and OpenXR is the one that makes the app *choose* from what the runtime offers. So the choice is made
/// by name, through `ash`'s constants rather than a magic number, and reported as what it is.
///
/// Eight bits first, because that is what every compositor must accept. A headset wants more than eight bits,
/// and that is a thing to add together with the tone map that makes it usable rather than a format to ask for
/// and hope for - so `Rgba16Float` is nameable here, and picking it is a decision for an app rather than a
/// default.
pub(crate) fn color_format(format: u32) -> wxr::ColorFormat {
    match format {
        f if f == ash::vk::Format::R8G8B8A8_SRGB.as_raw() as u32 => wxr::ColorFormat::Rgba8Srgb,
        f if f == ash::vk::Format::R8G8B8A8_UNORM.as_raw() as u32 => wxr::ColorFormat::Rgba8Unorm,
        f if f == ash::vk::Format::B8G8R8A8_SRGB.as_raw() as u32 => wxr::ColorFormat::Bgra8Srgb,
        f if f == ash::vk::Format::B8G8R8A8_UNORM.as_raw() as u32 => wxr::ColorFormat::Bgra8Unorm,
        f if f == ash::vk::Format::R16G16B16A16_SFLOAT.as_raw() as u32 => {
            wxr::ColorFormat::Rgba16Float
        }
        f if f == ash::vk::Format::A2B10G10R10_UNORM_PACK32.as_raw() as u32 => {
            wxr::ColorFormat::Rgb10a2Unorm
        }
        _ => wxr::ColorFormat::Unknown,
    }
}

/// Which OpenXR reference space a core one is.
pub(crate) fn reference_space(kind: wxr::SpaceKind) -> xr::ReferenceSpaceType {
    match kind {
        wxr::SpaceKind::Viewer => xr::ReferenceSpaceType::VIEW,
        wxr::SpaceKind::Local => xr::ReferenceSpaceType::LOCAL,
        wxr::SpaceKind::LocalFloor => xr::ReferenceSpaceType::LOCAL_FLOOR,
        // `STAGE` is the floor *and* the room the user walked in to define it, which is what bounded means.
        wxr::SpaceKind::BoundedFloor => xr::ReferenceSpaceType::STAGE,
        // An unbounded space is an extension; a runtime without it has `LOCAL`, and a caller that asked for
        // unbounded has got the space it asked for as closely as it exists.
        wxr::SpaceKind::Unbounded => xr::ReferenceSpaceType::LOCAL,
    }
}

/// The core's pose as OpenXR's, which is the direction an offset space is made in.
pub(crate) fn posef(pose: wxr::Pose) -> xr::Posef {
    xr::Posef {
        orientation: xr::Quaternionf {
            x: pose.orientation.x,
            y: pose.orientation.y,
            z: pose.orientation.z,
            w: pose.orientation.w,
        },
        position: xr::Vector3f {
            x: pose.position.x,
            y: pose.position.y,
            z: pose.position.z,
        },
    }
}

/// An OpenXR pose in the core's terms.
pub(crate) fn pose(pose: xr::Posef) -> wxr::Pose {
    wxr::Pose {
        position: Vec3::new(pose.position.x, pose.position.y, pose.position.z),
        orientation: Quat::from_xyzw(
            pose.orientation.x,
            pose.orientation.y,
            pose.orientation.z,
            pose.orientation.w,
        ),
    }
}

/// An OpenXR field of view in the core's terms.
///
/// OpenXR gives the four directions as angles from the centre, with up and right positive; the core keeps
/// them as the four openings, which is what a projection is built from.
pub(crate) fn field_of_view(fov: xr::Fovf) -> wxr::FieldOfView {
    wxr::FieldOfView {
        up: fov.angle_up,
        down: -fov.angle_down,
        left: fov.angle_left,
        right: -fov.angle_right,
    }
}
