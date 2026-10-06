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

/// The shader, inline because it is a page and a build script for a page is a build script.
///
/// `textureLoad` rather than `textureSample`: the depth a runtime hands over is `r32float` or `r16uint`, and
/// neither is a filterable format - so the texel is found by hand, which is honest about what the buffer is.
const SHADER: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
};

/// What the room's depth means: a transform from normalized view coordinates into the depth buffer's, and
/// what one value in it is worth in metres. `enabled` is 0 for a session with no depth to test against.
struct Occlusion {
    norm_depth_from_view: mat4x4<f32>,
    raw_value_to_meters: f32,
    enabled: u32,
    _pad: vec2<u32>,
};

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec3<f32>,
    /// Normalized view coordinates: 0..1, top-left, which is what the depth transform wants.
    @location(1) view: vec2<f32>,
    /// How far the vertex is in front of the eye, in metres - which is what a clip position's `w` is.
    @location(2) distance: f32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var real_depth: texture_2d<f32>;
@group(0) @binding(2) var<uniform> occlusion: Occlusion;

@vertex
fn vertex(@location(0) position: vec3<f32>, @location(1) colour: vec3<f32>) -> Vertex {
    var out: Vertex;
    let clip = camera.view_projection * vec4(position, 1.0);
    out.position = clip;
    out.colour = colour;
    out.distance = clip.w;
    out.view = clip.xy / clip.w * vec2(0.5, -0.5) + vec2(0.5, 0.5);
    return out;
}

@fragment
fn fragment(in: Vertex) -> @location(0) vec4<f32> {
    if (occlusion.enabled == 1u) {
        let view = (occlusion.norm_depth_from_view * vec4(in.view, 0.0, 1.0)).xy;
        let size = vec2<f32>(textureDimensions(real_depth));
        let texel = vec2<i32>(clamp(view * size, vec2<f32>(0.0), size - vec2<f32>(1.0)));
        let real = textureLoad(real_depth, texel, 0).r * occlusion.raw_value_to_meters;
        // A fragment farther away than what the room actually is in that direction is behind the room. A
        // buffer that says nothing (`0`) says nothing rather than hiding the whole scene behind the eye.
        if (real > 0.0 && in.distance > real) {
            discard;
        }
    }
    return vec4(in.colour, 1.0);
}
"#;

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

/// The room's depth, in the terms the shader tests against it: where the buffer is, and what it is worth.
#[derive(Clone, Copy, Debug)]
pub struct Occlusion {
    norm_depth_from_view: Mat4,
    raw_value_to_meters: f32,
}

impl Occlusion {
    /// The room as a session measured it.
    ///
    /// The transform is `DepthInfo`'s: normalized view coordinates into the depth buffer's own, which is where
    /// a fragment's place on the screen becomes a place in the buffer. The scale is what one raw value in the
    /// buffer is worth.
    pub fn from(info: wxr::DepthInfo) -> Self {
        Self {
            norm_depth_from_view: Mat4::from(info.norm_depth_buffer_from_norm_view.transform()),
            raw_value_to_meters: info.raw_value_to_meters,
        }
    }
}

/// The same, as the uniform the shader reads. Laid out by hand, because a uniform's layout is the shader's
/// and a mismatch is a picture that is wrong rather than a build that fails.
#[repr(C)]
#[derive(Clone, Copy)]
struct OcclusionUniform {
    norm_depth_from_view: [[f32; 4]; 4],
    raw_value_to_meters: f32,
    enabled: u32,
    /// Uniform buffers are laid out in 16-byte blocks; the two fields above are 8 bytes into one.
    _pad: [u32; 2],
}

/// Everything a scene needs that is the same for every eye.
pub struct Scene {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    camera: wgpu::Buffer,
    /// What the room's depth means, written per view.
    occlusion: wgpu::Buffer,
    /// A one-texel depth buffer to bind when a session measured nothing: a bind group needs a texture, and a
    /// shader with nothing to test against is told so by `enabled`.
    no_depth: wgpu::Texture,
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
            layout,
            // A hand's width to a room: an eye's field of view says nothing about either, because it is
            // about the lens. This is the renderer's guess and the day a scene has a floor, it is the scene's.
            near: planes().0,
            far: planes().1,
            depth,
        }
    }

    /// Draw the scene for one eye: its own field of view, its own place, the same geometry - and, when the
    /// session measured the room, its depth to test against.
    pub fn draw(
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

        // The room's depth for this view, or the one-texel stand-in and a 0 that says to draw everything.
        let (uniform, depth) = match occlusion {
            Some((texture, occlusion)) => (
                OcclusionUniform {
                    norm_depth_from_view: occlusion.norm_depth_from_view.to_cols_array_2d(),
                    raw_value_to_meters: occlusion.raw_value_to_meters,
                    enabled: 1,
                    _pad: [0; 2],
                },
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
            ),
            None => (
                OcclusionUniform {
                    norm_depth_from_view: Mat4::IDENTITY.to_cols_array_2d(),
                    raw_value_to_meters: 1.0,
                    enabled: 0,
                    _pad: [0; 2],
                },
                self.no_depth
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
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.occlusion.as_entire_binding(),
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
    (0.05, 100.0)
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
        // The shader's `Occlusion` is 64 bytes of matrix, a float, a uint, and a two-uint pad - and a mismatch
        // is a picture that is wrong rather than a build that fails, so it is pinned here.
        assert_eq!(std::mem::size_of::<OcclusionUniform>(), 64 + 4 + 4 + 8);
    }
}
