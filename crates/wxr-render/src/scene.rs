//! A triangle, drawn with each eye's own projection.
//!
//! The point of it is not the triangle. It is that the arithmetic below is the arithmetic above:
//! [`crate::projection::perspective`] turns the eye's four half-angles into a matrix,
//! [`crate::projection::view`] turns its pose into the other one, and the two are multiplied into the
//! uniform the vertex shader is given. A renderer whose projection is only ever tested and never used is a
//! renderer whose projection is a guess.
//!
//! One pipeline for every eye and a different matrix per eye, because that is the whole shape of stereo: the
//! eyes differ in where they are, not in what they are drawing.

use wxr::FieldOfView;

use crate::projection::{Depth, perspective};

/// The shader, inline because it is four lines and a build script for four lines is a build script.
const SHADER: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

@vertex
fn vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.view_projection * vec4(position, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4(0.25, 0.55, 1.0, 1.0);
}
"#;

/// A triangle in front of the origin, at about where a person sitting at a desk would be looking.
const TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.35, -1.5], [-0.35, -0.25, -1.5], [0.35, -0.25, -1.5]];

/// Everything a scene needs that is the same for every eye.
pub struct Scene {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    camera: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// The near and far planes, which a field of view does not carry: a projection needs both, and a
    /// runtime reports neither.
    near: f32,
    far: f32,
}

impl Scene {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wxr triangle"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wxr camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wxr camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wxr camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });

        let vertices = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("wxr triangle"),
                contents: bytemuck_cast(&TRIANGLE),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wxr triangle"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wxr triangle"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            vertices,
            camera,
            bind_group,
            // A hand's width to a room: an eye's field of view says nothing about either, because it is
            // about the lens. This is the renderer's guess and the day a scene has a floor, it is the scene's.
            near: planes().0,
            far: planes().1,
        }
    }

    /// Draw the scene for one eye: its own field of view, its own place, the same triangle.
    pub fn draw(&self, queue: &wgpu::Queue, view: &wxr::View, pass: &mut wgpu::RenderPass<'_>) {
        let camera = perspective(view.fov, self.near, self.far, Depth::ZeroToOne)
            * crate::projection::view(view.pose);
        queue.write_buffer(&self.camera, 0, bytemuck_cast(&camera));
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..3, 0..1);
    }
}

/// A field of view is what a runtime reports; a projection also needs the two planes, which it does not.
pub fn planes() -> (f32, f32) {
    (0.05, 100.0)
}

/// The triangles and the matrices as bytes.
///
/// `bytemuck` is not a dependency of this crate and the two types are plain, so this is the cast rather than
/// a crate to do it: a `[[f32; 3]; 3]` and a `Mat4` are both contiguous little-endian floats.
fn bytemuck_cast<T: Copy>(value: &T) -> &[u8] {
    // SAFETY: every type this is called with is `Copy` and has no padding - an array of `[f32; 3]` and a
    // `Mat4`, which is sixteen `f32`s. A type with padding or a pointer would make this a lie.
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// A field of view big enough to see the triangle from where an eye usually is.
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
}
