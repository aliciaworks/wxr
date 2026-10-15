/// The shader, as one raw string rather than as a file to be loaded.
///
/// It is a page of WGSL, and a build script for a page is a build script - reading it out of a `.wgsl` file
/// would trade a string for the program that produces the string - so it stays written in Rust. It is in a
/// file of its own only because this project counts lines, and this is a block of them that belongs together
/// and belongs to nothing else here.
///
/// `textureLoad` rather than `textureSample`: the depth a runtime hands over is `r32float` or `r16uint`, and
/// neither is a filterable format - so the texel is found by hand, which is honest about what the buffer is.
pub(super) const SHADER: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
};

/// What the room's depth means: a transform from normalized view coordinates into the depth buffer's, what
/// its values *are*, and what to do with them. `enabled` is 0 for a session with no depth to test against,
/// which is a different thing from a buffer that says nothing - see `kind`.
struct Occlusion {
    norm_depth_from_view: mat4x4<f32>,
    /// 1 = linear (`metres = value * to_meters`), 2 = window depth over the two planes below, 3 = unknown.
    kind: u32,
    enabled: u32,
    /// The three numbers the two encodings take, in the order `kind` says they are used in.
    linear_scale: f32,
    near_z: f32,
    far_z: f32,
    _spare: u32,
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

/// How far away the room is at a texel, in metres - or `0.0` when nothing says.
///
/// Two encodings and two functions, and the third case is neither: a buffer whose meaning was never stated
/// has no distance to report. `0.0` is what the test below already treats as "says nothing", which is why it
/// survives the change of shape. A session with no depth at all never reaches this - the caller checks
/// `enabled` first - so `0.0` from here always means "this buffer does not say", never "the room is here".
fn metres(value: f32) -> f32 {
    if (occlusion.kind == 1u) {
        return value * occlusion.linear_scale;
    }
    if (occlusion.kind == 2u) {
        // Window depth over the frustum's two planes: the inverse of the projection that wrote it, with the
        // window-space ends at 0 and 1, which is what the extension reports them as - see
        // `wxr::DepthEncoding::Window` for the forward direction and for why those two are given.
        let near = occlusion.near_z;
        let far = occlusion.far_z;
        // `far <= near` is an infinite frustum, not a reversed one - `XR_META_environment_depth` says so in
        // as many words - and the finite formula below would quietly produce a number from a pair of planes
        // it cannot represent. A value of 1 is the far plane, which for an infinite one is no distance at
        // all: the guard keeps that from dividing by zero, and an enormous distance occludes nothing.
        if (far <= near) {
            return near / max(1.0 - value, 1e-6);
        }
        return near * far / (far - value * (far - near));
    }
    return 0.0;
}

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
        let real = metres(textureLoad(real_depth, texel, 0).r);
        // A fragment farther away than what the room actually is in that direction is behind the room. A
        // buffer that says nothing says nothing rather than hiding the whole scene behind the eye - which is
        // `metres`'s `0.0`, and the reason `0.0` is what it returns for an encoding it does not know.
        if (real > 0.0 && in.distance > real) {
            discard;
        }
    }
    return vec4(in.colour, 1.0);
}
"#;
