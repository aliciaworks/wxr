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

/// An OpenXR *surface* pose in the core's terms, and the one place the two conventions differ.
///
/// OpenXR puts a surface's normal on the pose's **+Z**: a plane's extent is measured along the pose's X and
/// Y, and a raycast hit's normal is "rotating {0, 0, 1} by this orientation". The core puts it on **+Y**,
/// which is what WebXR and ARKit say and what a caller has been reading on every other backend. So the
/// orientation turns and the position does not: a surface's centre is where it is whichever way the axes
/// are named.
///
/// That turn is one rotation about the pose's own X, and the spec spells out exactly this conversion in
/// `XR_MSFT_scene_understanding`:
///
/// > OpenXR uses an X-Y plane with +Z as the plane normal but other APIs may use an X-Z plane with +Y as
/// > the plane normal. The X-Y plane can be converted to an X-Z plane by rotating -pi/2 radians around the
/// > +X axis.
///
/// Which is what this is, written the other way round: the core's frame is OpenXR's turned by +pi/2, so that
/// the core's +Y lands where OpenXR's +Z was. Only +pi/2 and not -pi/2, and the direction is pinned by the
/// test below rather than by the prose - the note describes turning the plane, this turns the frame the
/// plane is described in.
pub(crate) fn surface(pose: wxr::Pose) -> wxr::Pose {
    wxr::Pose {
        position: pose.position,
        orientation: pose.orientation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
    }
}

#[cfg(test)]
mod tests {
    use wxr::glam::Vec3;

    use super::*;

    /// How far off a direction is allowed to be, which is one part in a million and no more.
    const SLACK: f32 = 1e-6;

    /// A floor OpenXR found has its normal on +Z, and the core asks for it on +Y.
    ///
    /// This is the whole of what `surface` is for, and it is the test that would catch the rotation being
    /// the wrong way round - which turns every surface the wrong way up and looks like correct code.
    #[test]
    fn an_openxr_surface_normal_comes_out_on_the_cores_y() {
        let core = surface(wxr::Pose::IDENTITY);
        assert!((core.orientation * Vec3::Y - Vec3::Z).length() < SLACK);
    }

    /// And it points *away* from the surface the same way it did: a floor's normal is still up, and a
    /// conversion that flipped it would put every floor on the ceiling.
    #[test]
    fn a_surface_keeps_which_way_it_faces() {
        // OpenXR's normal for this plane is +Z, so its core normal has to be +Z too.
        let flipped = wxr::Pose {
            position: wxr::glam::Vec3::ZERO,
            orientation: Quat::from_rotation_y(std::f32::consts::PI),
        };
        let core = surface(flipped);
        assert!((core.orientation * Vec3::Y - Vec3::NEG_Z).length() < SLACK);
    }

    /// A surface's centre is where it is whichever way the axes are named.
    #[test]
    fn a_surface_keeps_where_its_centre_is() {
        let pose = wxr::Pose {
            position: Vec3::new(1.0, 2.0, 3.0),
            orientation: Quat::from_rotation_z(0.7),
        };
        let core = surface(pose);
        assert_eq!(core.position, Vec3::new(1.0, 2.0, 3.0));
        assert!(
            (core.orientation
                - pose.orientation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
            .length()
                < SLACK
        );
    }

    /// The core measures a plane's extent in its own X and Z, and OpenXR measures it in its X and Y - so the
    /// two numbers carry over as they are, which is the one thing that makes the extents worth checking.
    #[test]
    fn the_in_plane_x_axis_is_x_in_both() {
        let core = surface(wxr::Pose::IDENTITY);
        assert!((core.orientation * Vec3::X - Vec3::X).length() < SLACK);
        // And the in-plane axes stay at right angles: the rotation is not a shear.
        assert!(
            (core.orientation * Vec3::X)
                .dot(core.orientation * Vec3::Z)
                .abs()
                < SLACK
        );
    }
}
