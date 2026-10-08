//! Reading ARKit's mesh anchors as the core's meshes.
//!
//! Separated from the provider that fetches them because they answer two questions: the provider is where the
//! triangles come from, and this is what a buffer of `float3` and a list of triangle indices are once they are
//! Rust's. Every reader hedges the same way - a format it does not know is an empty mesh rather than a mesh of
//! garbage - because the one thing worse than a room that is not drawn is a room drawn wrong.

use std::ffi::c_void;
use std::time::Duration;

use objc2_ar_kit::{
    ar_geometry_element_t, ar_geometry_source_t, ar_mesh_anchor_t, ar_mesh_classification_t,
    ar_mesh_geometry_t,
};
use objc2_metal_visionos::{MTLBuffer, MTLVertexFormat};
use wxr::glam::{Mat4, Vec3};

use crate::sys;

/// The enumerator `meshes` hands to ARKit: one mesh into the `Vec` the context points at.
///
/// # Safety
///
/// `context` must be the `*mut Vec<wxr::Mesh>` the caller passed, and `anchor` a live mesh anchor.
pub(super) unsafe extern "C-unwind" fn collect_mesh(
    context: *mut c_void,
    anchor: &ar_mesh_anchor_t,
) -> bool {
    // SAFETY: the caller guarantees both.
    unsafe {
        (*context.cast::<Vec<wxr::Mesh>>()).push(mesh_of(anchor));
    }
    true
}

/// One mesh anchor, as the core's [`wxr::Mesh`].
///
/// # Safety
///
/// `anchor` must be a live mesh anchor.
unsafe fn mesh_of(anchor: &ar_mesh_anchor_t) -> wxr::Mesh {
    // SAFETY: the caller guarantees the anchor is live; the transform is where ARKit put it, the geometry is
    // the triangles it traced for it, and the identifier is a UUID it fills in.
    unsafe {
        let mut identifier = [0u8; 16];
        ar_mesh_anchor_t::identifier(anchor, &mut identifier);
        let transform = sys::ar_anchor_get_origin_from_anchor_transform(
            std::ptr::from_ref(anchor).cast::<c_void>(),
        );
        let (_, orientation, position) =
            Mat4::from_cols_array(&transform.0).to_scale_rotation_translation();
        let geometry = ar_mesh_anchor_t::geometry(anchor);
        wxr::Mesh {
            // The anchor's UUID folded to the name the core carries, the same fold `planes` makes: the same
            // 32 bits for as long as the anchor lives, which is what a stable id has to be.
            id: u32::from_le_bytes([identifier[0], identifier[1], identifier[2], identifier[3]]),
            pose: wxr::Pose {
                position,
                orientation,
            },
            vertices: read_vectors(&ar_mesh_geometry_t::vertices(&geometry)),
            indices: read_indices(&ar_mesh_geometry_t::faces(&geometry)),
            kind: kind_of(&geometry),
            // The runtime's own clock, in seconds - the same kind of number the draft's `lastChangedTime` is:
            // when this patch of the room last changed, as the runtime counts time.
            last_changed: Duration::from_secs_f64(ar_mesh_anchor_t::timestamp(anchor).max(0.0)),
        }
    }
}

/// Read a `float3` source as vectors, honouring the offset and the stride ARKit gives it with.
///
/// # Safety
///
/// `source` must be a live geometry source whose buffer outlives the read.
unsafe fn read_vectors(source: &ar_geometry_source_t) -> Vec<Vec3> {
    // SAFETY: the source is live; its buffer is the runtime's and is read here rather than kept.
    unsafe {
        // The one format: ARKit's mesh positions are `float3`, and a format this does not know is a mesh left
        // empty rather than a mesh of garbage - the same trade the renderer makes about a colour format it has
        // not learned, and for the same reason.
        if ar_geometry_source_t::format(source) != MTLVertexFormat::Float3 {
            return Vec::new();
        }
        let count = ar_geometry_source_t::count(source);
        let stride = ar_geometry_source_t::stride(source);
        let offset = ar_geometry_source_t::offset(source);
        let buffer = ar_geometry_source_t::buffer(source);
        let base = buffer.contents().cast::<u8>();
        if base.is_null() {
            return Vec::new();
        }
        (0..count)
            .map(|i| {
                let at = base.add(offset + i * stride).cast::<f32>();
                // SAFETY: the runtime says there are `count` of these, `stride` apart, three floats each.
                Vec3::new(*at, *at.add(1), *at.add(2))
            })
            .collect()
    }
}

/// Read a face list as triangle indices.
///
/// # Safety
///
/// `faces` must be the live element of a mesh geometry whose buffer outlives the read.
unsafe fn read_indices(faces: &ar_geometry_element_t) -> Vec<u32> {
    // SAFETY: as above.
    unsafe {
        let count = ar_geometry_element_t::count(faces);
        let bytes = ar_geometry_element_t::bytes_per_index(faces);
        let buffer = ar_geometry_element_t::buffer(faces);
        let base = buffer.contents().cast::<u8>();
        if base.is_null() {
            return Vec::new();
        }
        // Two widths and no others: an index list is 16- or 32-bit, and a third is a mesh this cannot read -
        // and empty is the honest answer to that, as it is for a format above.
        let read: fn(*const u8) -> u32 = match bytes {
            2 => |at| u16::from_ne_bytes([*at, *at.add(1)]) as u32,
            4 => |at| u32::from_ne_bytes([*at, *at.add(1), *at.add(2), *at.add(3)]),
            _ => return Vec::new(),
        };
        (0..count).map(|i| read(base.add(i * bytes))).collect()
    }
}

/// What ARKit says the mesh is, as the core's word.
///
/// ARKit classifies per *face* and the core's label is per mesh - the draft's `semanticLabel` is one per
/// `XRMesh` - so a per-face answer has to become a per-mesh one. It is the most common of the faces: asking
/// what this patch of room mostly is, which is the honest reading of the two, and not the first face, which
/// would make the label depend on which triangle the runtime happened to write down first.
///
/// # Safety
///
/// `geometry` must be a live mesh geometry.
unsafe fn kind_of(geometry: &ar_mesh_geometry_t) -> wxr::MeshKind {
    // SAFETY: the geometry is live, and the classification source is the runtime's.
    unsafe {
        let Some(source) = ar_mesh_geometry_t::classification(geometry) else {
            return wxr::MeshKind::None;
        };
        let mut counts: Vec<(i64, usize)> = Vec::new();
        for face in read_classifications(&source) {
            match counts.iter_mut().find(|(value, _)| *value == face) {
                Some((_, count)) => *count += 1,
                None => counts.push((face, 1)),
            }
        }
        // The most common; a tie is two labels that are both in the mesh, and either is as true as the other.
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(raw, _)| kind_from_raw(raw))
            .unwrap_or(wxr::MeshKind::None)
    }
}

/// The classification of each face, as the number ARKit stored for it.
///
/// # Safety
///
/// `source` must be a live geometry source whose buffer outlives the read.
unsafe fn read_classifications(source: &ar_geometry_source_t) -> Vec<i64> {
    // SAFETY: the source is live; its buffer is the runtime's and is read here rather than kept.
    unsafe {
        let count = ar_geometry_source_t::count(source);
        let stride = ar_geometry_source_t::stride(source);
        let offset = ar_geometry_source_t::offset(source);
        let format = ar_geometry_source_t::format(source);
        let buffer = ar_geometry_source_t::buffer(source);
        let base = buffer.contents().cast::<u8>();
        if base.is_null() {
            return Vec::new();
        }
        // `ar_mesh_classification_t` is an integer enum, and which width the runtime wrote is what its format
        // says - so each of the integer formats is read as itself and widened, and a float or a vector is a
        // source this cannot read.
        let read: fn(*const u8) -> i64 = match format {
            f if f == MTLVertexFormat::UChar => |at| *at as i64,
            f if f == MTLVertexFormat::Char => |at| *at.cast::<i8>() as i64,
            f if f == MTLVertexFormat::UShort => |at| u16::from_ne_bytes([*at, *at.add(1)]) as i64,
            f if f == MTLVertexFormat::Short => |at| i16::from_ne_bytes([*at, *at.add(1)]) as i64,
            f if f == MTLVertexFormat::UInt => {
                |at| u32::from_ne_bytes([*at, *at.add(1), *at.add(2), *at.add(3)]) as i64
            }
            f if f == MTLVertexFormat::Int => {
                |at| i32::from_ne_bytes([*at, *at.add(1), *at.add(2), *at.add(3)]) as i64
            }
            _ => return Vec::new(),
        };
        (0..count)
            .map(|i| read(base.add(offset + i * stride)))
            .collect()
    }
}

/// ARKit's classification number as the core's word.
///
/// The numbers come from ARKit's own constants rather than being written out here, so a runtime that
/// renumbered them - which it will not - would still be read correctly.
fn kind_from_raw(raw: i64) -> wxr::MeshKind {
    let value = raw as isize;
    match value {
        v if v == ar_mesh_classification_t::wall.0 => wxr::MeshKind::Wall,
        v if v == ar_mesh_classification_t::floor.0 => wxr::MeshKind::Floor,
        v if v == ar_mesh_classification_t::ceiling.0 => wxr::MeshKind::Ceiling,
        v if v == ar_mesh_classification_t::table.0 => wxr::MeshKind::Table,
        v if v == ar_mesh_classification_t::seat.0 => wxr::MeshKind::Seat,
        v if v == ar_mesh_classification_t::door.0 => wxr::MeshKind::Door,
        v if v == ar_mesh_classification_t::window.0 => wxr::MeshKind::Window,
        v if v == ar_mesh_classification_t::cabinet.0 => wxr::MeshKind::Cabinet,
        v if v == ar_mesh_classification_t::bed.0 => wxr::MeshKind::Bed,
        v if v == ar_mesh_classification_t::plant.0 => wxr::MeshKind::Plant,
        v if v == ar_mesh_classification_t::stairs.0 => wxr::MeshKind::Stairs,
        v if v == ar_mesh_classification_t::tv.0 => wxr::MeshKind::Tv,
        v if v == ar_mesh_classification_t::home_appliance.0 => wxr::MeshKind::HomeAppliance,
        // `ar_mesh_classification_t::none`, and anything a newer runtime adds: a mesh with no label, which is
        // a room to collide with all the same.
        _ => wxr::MeshKind::None,
    }
}
