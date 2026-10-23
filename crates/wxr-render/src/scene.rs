//! Two triangles, drawn with each eye's own projection and its own depth - and, when the session measured the
//! room, tested against it.
//!
//! The point of them is not the triangles. It is that the arithmetic below is the arithmetic above:
//! [`crate::projection::perspective`] turns the eye's four half-angles into a matrix,
//! [`crate::projection::view`] turns its pose into the other one, and the two are multiplied into the
//! uniform the vertex shader is given. A renderer whose projection is only ever tested and never used is a
//! renderer whose projection is a guess.
//!
//! The second triangle exists for the depth buffer's sake, and so does the order they are drawn in. One sits
//! in front of the other and the *near* one is drawn *first* - so a pass without a depth buffer would let the
//! far one paint over it, and a pass with one shows the near triangle, which is what a person in a room
//! expects. Their colours differ for the same reason: an occlusion nobody can see in a picture is an
//! occlusion nobody has tested.
//!
//! The *room's* depth is the other half of that: when a session hands over a measurement of the real world,
//! the fragment shader throws away every fragment that is behind it. That is what makes a virtual object go
//! behind a real table instead of through it - and it is off unless a session gave a buffer, because a scene
//! with nothing to test against is a scene that draws everything.
//!
//! One pipeline for every eye and a different matrix per eye, because that is the whole shape of stereo: the
//! eyes differ in where they are, not in what they are drawing.

use wxr::FieldOfView;
use wxr::glam::Mat4;

use crate::projection::{Depth, perspective};

// The shader is a page of WGSL in a crate that is Rust, and it is the part of this module whose length
// has nothing to do with anything else here - which is the size rule's own argument for a split rather
// than a longer file. What it says about `textureLoad` is on it, where it is read.
mod shader;

use shader::SHADER;

/// A triangle in front of the origin and the same shape behind it, as `[x, y, z, r, g, b]` per vertex.
///
/// Roughly where a person sitting at a desk would be looking, and far enough apart that the depth buffer has
/// something to decide: with no buffer at all the second triangle would win, because it is drawn second.
pub const TRIANGLES: [[f32; 6]; 6] = [
    [0.0, 0.35, -1.0, 0.25, 0.55, 1.0],
    [-0.35, -0.25, -1.0, 0.25, 0.55, 1.0],
    [0.35, -0.25, -1.0, 0.25, 0.55, 1.0],
    [0.0, 0.35, -2.5, 1.0, 0.35, 0.2],
    [-0.35, -0.25, -2.5, 1.0, 0.35, 0.2],
    [0.35, -0.25, -2.5, 1.0, 0.35, 0.2],
];

/// The colour of the near triangle, which is what a picture of this scene should show where they overlap.
pub const NEAR: [f32; 3] = [0.25, 0.55, 1.0];

/// The room's depth, in the terms the shader tests against it: where the buffer is, and what its values are.
///
/// Kept as a tag plus all three numbers rather than as one, because the two encodings do not share their
/// arguments: a linear buffer has a scale and no planes, a window one has planes and no scale. Carrying both
/// is not redundancy, it is the shape the uniform is - and a reader who sees `near_z` filled in on a linear
/// buffer can see at once that this is the unused half rather than a second meaning.
#[derive(Clone, Copy, Debug)]
pub struct Occlusion {
    norm_depth_from_view: Mat4,
    /// What the shader's `kind` is: 1 linear, 2 window, 3 unknown.
    kind: u32,
    linear_scale: f32,
    near_z: f32,
    far_z: f32,
}

impl Occlusion {
    /// The room as a session measured it.
    ///
    /// The transform is `DepthInfo`'s: normalized view coordinates into the depth buffer's own, which is where
    /// a fragment's place on the screen becomes a place in the buffer. The rest is [`wxr::DepthEncoding`],
    /// translated rather than interpreted - what the encoding says goes in, and the arithmetic is the
    /// shader's, once, in `metres`, rather than split between here and there.
    pub fn from(info: wxr::DepthInfo) -> Self {
        let (kind, linear_scale, near_z, far_z) = match info.encoding {
            wxr::DepthEncoding::Linear { to_meters } => (1, to_meters, 0.0, 0.0),
            wxr::DepthEncoding::Window { near_z, far_z } => (2, 0.0, near_z, far_z),
            wxr::DepthEncoding::Unknown => (3, 0.0, 0.0, 0.0),
        };
        Self {
            norm_depth_from_view: Mat4::from(info.norm_depth_buffer_from_norm_view.transform()),
            kind,
            linear_scale,
            near_z,
            far_z,
        }
    }
}

/// The same, as the uniform the shader reads. Laid out by hand, because a uniform's layout is the shader's
/// and a mismatch is a picture that is wrong rather than a build that fails.
///
/// The field order is the shader's and not this file's taste: `kind` and `enabled` come first because they
/// are what the shader branches on, the three numbers follow as the arguments whichever branch takes, and the
/// padding is there because a uniform buffer is laid out in 16-byte blocks and this comes to 96 with it and
/// 92 without.
#[repr(C)]
#[derive(Clone, Copy)]
struct OcclusionUniform {
    norm_depth_from_view: [[f32; 4]; 4],
    kind: u32,
    enabled: u32,
    linear_scale: f32,
    near_z: f32,
    far_z: f32,
    /// 1 when [`Scene`]'s typed depth binding holds the room, 0 when its float binding does.
    depth_is_depth: u32,
    _pad: [u32; 2],
}

impl OcclusionUniform {
    /// The uniform for a session with no measured room, which is what the stand-in texture is bound beside.
    ///
    /// `enabled` 0 is the whole answer, and the three numbers are set anyway rather than left as whatever
    /// memory the struct was built from - a uniform that reads a stale `near_z` on a path that does not use
    /// it is still a uniform that read it.
    fn off() -> Self {
        Self {
            norm_depth_from_view: Mat4::IDENTITY.to_cols_array_2d(),
            kind: 0,
            enabled: 0,
            linear_scale: 1.0,
            near_z: 0.0,
            far_z: 0.0,
            depth_is_depth: 0,
            _pad: [0; 2],
        }
    }
}

/// Everything a scene needs that is the same for every eye.
pub struct Scene {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    camera: wgpu::Buffer,
    /// What the room's depth means, written per view.
    occlusion: wgpu::Buffer,
    /// A one-texel float buffer to bind when a session measured nothing, or measured into a depth format:
    /// a bind group needs both textures, and a shader with nothing to test against is told so by `enabled`.
    no_depth: wgpu::Texture,
    /// The same stand-in for the typed binding, which is the one OpenXR's `D16_UNORM` room arrives on.
    no_depth_typed: wgpu::Texture,
    layout: wgpu::BindGroupLayout,
    /// The near and far planes, which a field of view does not carry: a projection needs both, and a
    /// runtime reports neither.
    near: f32,
    far: f32,
    /// Where a projection puts its depth range, which is the target's to insist on rather than the scene's.
    /// visionOS is why this is here: a pass drawn into a `CompositorServices` drawable has to be reverse-Z,
    /// and a renderer that had decided otherwise for every platform could not draw on one.
    depth: Depth,
}

impl Scene {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, depth: Depth) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wxr scene"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wxr camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let occlusion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wxr occlusion"),
            size: std::mem::size_of::<OcclusionUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let no_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wxr no depth"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let no_depth_typed = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wxr no typed depth"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wxr scene"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // A depth buffer is never filterable, so this is a plain texture with no sampler beside it and
                // the shader reads it with `textureLoad`.
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // The typed half: a depth format such as OpenXR's `D16_UNORM` cannot be bound as a float
                // texture, so the room is bound here when that is what it is and the shader selects it.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let vertices = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("wxr triangles"),
                contents: bytemuck_cast(&TRIANGLES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wxr scene"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // The pipeline declares the depth buffer the pass has to attach, and how it compares - which is the
        // one thing the projection convention decides about a pipeline.
        let depth_state = crate::depth_state(depth);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wxr scene"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(depth_state.compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            vertices,
            camera,
            occlusion,
            no_depth,
            no_depth_typed,
            layout,
            // A hand's width to a room: an eye's field of view says nothing about either, because it is
            // about the lens. This is the renderer's guess and the day a scene has a floor, it is the scene's.
            near: planes().0,
            far: planes().1,
            depth,
        }
    }
}

impl crate::Draw for Scene {
    /// Draw the scene for one eye: its own field of view, its own place, the same geometry - and, when the
    /// session measured the room, its depth to test against.
    ///
    /// This is the fixture's implementation of [`crate::Draw`], the seam an app writes its own scenes
    /// against: nothing here is the renderer's, and a real scene replaces it rather than extends it.
    fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wxr::View,
        occlusion: Option<&(wgpu::Texture, Occlusion)>,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        let camera = perspective(view.fov, self.near, self.far, self.depth)
            * crate::projection::view(view.pose);
        queue.write_buffer(&self.camera, 0, bytemuck_cast(&camera));

        // The room's depth for this view, or the one-texel stand-in and an `enabled` of 0 that draws
        // everything. The two are different absences - no measurement, and a measurement that does not say -
        // and they land on the same branch below only because both mean the same thing to a fragment.
        let (uniform, float_depth, typed_depth) = match occlusion {
            Some((texture, occlusion)) => {
                // The two sample types are not interchangeable in wgpu: a depth format binds as a depth
                // texture, and `r32float` binds as an unfilterable float one. The shader is told which of
                // the two bindings holds the room, and the other gets a one-texel stand-in.
                let typed = texture.format().is_depth_stencil_format();
                (
                    OcclusionUniform {
                        norm_depth_from_view: occlusion.norm_depth_from_view.to_cols_array_2d(),
                        kind: occlusion.kind,
                        enabled: 1,
                        linear_scale: occlusion.linear_scale,
                        near_z: occlusion.near_z,
                        far_z: occlusion.far_z,
                        depth_is_depth: typed as u32,
                        _pad: [0; 2],
                    },
                    if typed {
                        self.no_depth
                            .create_view(&wgpu::TextureViewDescriptor::default())
                    } else {
                        texture.create_view(&wgpu::TextureViewDescriptor::default())
                    },
                    if typed {
                        texture.create_view(&wgpu::TextureViewDescriptor::default())
                    } else {
                        self.no_depth_typed
                            .create_view(&wgpu::TextureViewDescriptor::default())
                    },
                )
            }
            None => (
                OcclusionUniform::off(),
                self.no_depth
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                self.no_depth_typed
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ),
        };
        queue.write_buffer(&self.occlusion, 0, bytemuck_cast(&uniform));

        // Made per view and not kept: the depth belongs to the frame it came with, so a bind group holding it
        // would hold a texture that is gone by the next one.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wxr scene"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&float_depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.occlusion.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&typed_depth),
                },
            ],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..TRIANGLES.len() as u32, 0..1);
    }
}

/// A field of view is what a runtime reports; a projection also needs the two planes, which it does not.
pub fn planes() -> (f32, f32) {
    // 0.1 is the closest near plane a CompositorServices drawable accepts, so it is the shared floor
    // rather than a scene choice: a projection built with anything nearer is refused by the compositor.
    (0.1, 100.0)
}

/// The triangles and the matrices as bytes.
///
/// `bytemuck` is not a dependency of this crate and the types are plain, so this is the cast rather than a
/// crate to do it: a `[[f32; 6]; 6]` and a `Mat4` are both contiguous little-endian floats.
fn bytemuck_cast<T: Copy>(value: &T) -> &[u8] {
    // SAFETY: every type this is called with is `Copy` and has no padding - an array of `[f32; 6]`, a `Mat4`
    // which is sixteen `f32`s, and the occlusion uniform whose padding is written out. A type with padding
    // this did not account for would make this a lie.
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// A field of view big enough to see the triangles from where an eye usually is.
pub const DEFAULT_FOV: FieldOfView = FieldOfView {
    up: 0.9,
    down: 0.9,
    left: 0.9,
    right: 0.9,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::angles;

    #[test]
    fn the_default_field_of_view_survives_the_round_trip() {
        let back = angles(perspective(DEFAULT_FOV, 0.05, 100.0, Depth::ZeroToOne));
        assert!((back.up - DEFAULT_FOV.up).abs() < 1e-4);
        assert!((back.left - DEFAULT_FOV.left).abs() < 1e-4);
    }

    #[test]
    fn a_triangle_at_the_far_plane_is_behind_a_triangle_at_the_near_one() {
        // The two depths the projection has to order, which is the one thing a depth range is for.
        let (near, far) = planes();
        let matrix = perspective(DEFAULT_FOV, near, far, Depth::ZeroToOne);
        let at = |z: f32| (matrix * wxr::glam::Vec4::new(0.0, 0.0, z, 1.0)).z;
        assert!(at(-near) < at(-far), "nearer is smaller");
    }

    #[test]
    fn the_occlusion_uniform_is_what_the_shader_says_it_is() {
        // The shader's `Occlusion` is 64 bytes of matrix, then `kind` and `enabled` and the three numbers
        // the two encodings take - six 4-byte fields - and a spare plus a two-uint pad to carry it to the
        // next 16-byte block. A mismatch is a picture that is wrong rather than a build that fails, so it is
        // pinned here, and the field *order* is the shader's too: `kind` before the numbers it selects.
        assert_eq!(std::mem::size_of::<OcclusionUniform>(), 64 + 6 * 4 + 8);
        assert_eq!(
            std::mem::offset_of!(OcclusionUniform, kind),
            64,
            "the shader branches on `kind` immediately after the matrix"
        );
        assert_eq!(
            std::mem::offset_of!(OcclusionUniform, _pad),
            88,
            "the pad starts on an 8-byte boundary, which is what a vec2 needs"
        );
    }
}
