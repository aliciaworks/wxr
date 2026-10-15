//! What a room's depth does to a fragment, for both ways a depth buffer can mean a distance.
//!
//! Split out of `mod.rs` for the size rule, and it is the half that is about a *number* rather than about
//! drawing: [`wxr::DepthEncoding`] says a buffer is either a distance times a scale or a projection between
//! two planes, and the shader has to get both right rather than one of them. Two encodings is also the shape
//! this file is tested in - each one gets the case that would pass if the other's arithmetic had been used.

use super::*;

/// A room at `value` with `encoding` said, as a one-texel depth buffer.
///
/// A shader that reads a depth buffer needs nothing more than a value and the metadata that says what it
/// means, and a one-texel buffer is a room that is the same distance in every direction - which is all a test
/// needs. Everything else in this file is a pair of those with a different `value` and a different encoding.
fn texel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    value: f32,
    encoding: wxr::DepthEncoding,
) -> (wgpu::Texture, scene::Occlusion) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test room"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &value.to_le_bytes(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let info = wxr::DepthInfo {
        size: wxr::Extent2d::new(1, 1),
        encoding,
        norm_depth_buffer_from_norm_view: wxr::Pose::IDENTITY,
    };
    (texture, scene::Occlusion::from(info))
}

/// A room of one distance, as a linear buffer whose value already is that distance.
fn room(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    metres: f32,
) -> (wgpu::Texture, scene::Occlusion) {
    texel(
        device,
        queue,
        metres,
        wxr::DepthEncoding::Linear { to_meters: 1.0 },
    )
}

/// The forward direction of a window-depth projection: how far out a point is, given where between the two
/// planes the buffer said it was.
///
/// Written from the projection rather than by inverting the shader's own formula, because two formulas that
/// are inverses of each other agree whether or not either of them is right. This one is what a depth buffer
/// *is*: the near plane writes 0 and the far plane 1, and the number between them is hyperbolic in distance
/// because that is what keeps precision where the eye is.
///
/// At `near` it is exactly 0 and at `far` exactly 1, which the test below asserts numerically before it asks
/// a GPU anything - those two are what the extension states, and they are what pins the rest.
fn window_value(near: f32, far: f32, metres: f32) -> f32 {
    far * (metres - near) / (metres * (far - near))
}

/// Whether the near triangle is what comes back at the centre, which is the only thing two outcomes can say.
///
/// It is the same comparison [`super::the_nearer_triangle_is_the_one_that_shows`] makes, and the reason
/// there are two outcomes rather than three: the far triangle is the same shape farther away, so it projects
/// *smaller* and is entirely covered by the near one. A sample point where only the far triangle were
/// visible does not exist, and what distinguishes the cases here is therefore whether the near one survived.
fn standing(pixel: [u8; 3]) -> bool {
    let near = [
        (scene::NEAR[0] * 255.0).round() as u8,
        (scene::NEAR[1] * 255.0).round() as u8,
        (scene::NEAR[2] * 255.0).round() as u8,
    ];
    pixel
        .iter()
        .zip(near)
        .all(|(got, want)| got.abs_diff(want) <= 2)
}

/// The room's depth is what makes a fragment behind it disappear, and this is the test that says the shader
/// does it rather than that it compiles.
///
/// The scene's triangles are 1 m and 2.5 m in front of the eye, so a room at half a metre is in front of both
/// and a room at five metres is behind them - and the picture has to differ from the one without a room in the
/// first case and be exactly it in the second. That "exactly" is the half worth having: an occlusion test that
/// only checked the first case would pass for a shader that discarded everything.
#[test]
fn a_fragment_behind_the_room_is_not_drawn() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let drawn = super::centre_pixel(&device, &queue, Depth::ZeroToOne);
    assert!(
        standing(drawn),
        "with no room the near triangle is what shows: {drawn:?}"
    );

    let blocked = super::centre_pixel_with(
        &device,
        &queue,
        Depth::ZeroToOne,
        Some(room(&device, &queue, 0.5)),
    );
    assert_ne!(
        blocked, drawn,
        "a room in front of everything hides the scene"
    );

    let behind = super::centre_pixel_with(
        &device,
        &queue,
        Depth::ZeroToOne,
        Some(room(&device, &queue, 5.0)),
    );
    assert_eq!(behind, drawn, "a room behind everything changes nothing");
}

/// A linear buffer's scale is applied before anything is compared, which is the whole of what the scale is.
///
/// The same value with two scales is a room two distances apart, and this is the case the old shape of
/// [`wxr::DepthInfo`] could not have expressed: a scale that is not 1 is what `rawValueToMeters` exists for,
/// and a shader that ignored it would put every room ten times nearer than it is.
#[test]
fn a_linear_buffers_scale_is_what_turns_its_value_into_metres() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    let drawn = super::centre_pixel(&device, &queue, Depth::ZeroToOne);

    // A value of 5 at a tenth of a metre each: a room at half a metre, which hides everything.
    let scaled = texel(
        &device,
        &queue,
        5.0,
        wxr::DepthEncoding::Linear { to_meters: 0.1 },
    );
    let blocked = super::centre_pixel_with(&device, &queue, Depth::ZeroToOne, Some(scaled));
    assert_ne!(
        blocked, drawn,
        "5 values at 0.1 m each is a room at 0.5 m, in front of everything"
    );

    // And the same buffer with the scale as it comes: a room at five metres, which hides nothing. Without
    // this the first half would pass for a shader that discarded every fragment.
    let unscaled = texel(
        &device,
        &queue,
        5.0,
        wxr::DepthEncoding::Linear { to_meters: 1.0 },
    );
    let behind = super::centre_pixel_with(&device, &queue, Depth::ZeroToOne, Some(unscaled));
    assert_eq!(behind, drawn, "5 metres is behind the whole scene");
}

/// The same scene against a window-depth buffer, which is what an OpenXR runtime hands over.
///
/// This is the arithmetic [`wxr::DepthEncoding::Window`] is for and the reason the type is an enum: the value
/// in the buffer is not a distance, it is a projection of one, and multiplying it by anything compares a
/// fraction against a distance. Each row below is chosen so that exactly one of the shader's mistakes would
/// flip it:
///
/// * two rows differ **only in `near_z`** and two differ **only in `far_z`**, so a shader that read one plane
///   and ignored the other fails on the pair;
/// * two rows bracket the near triangle's own 1 m with values whose *linear* reading would both be far
///   behind it, so a shader that interpolated the planes instead of hyperbolically between them fails;
/// * two are `far_z < near_z`, which `XR_META_environment_depth` says is an infinite frustum rather than a
///   reversed one - the branch where a division by zero and a plausible-looking number are both waiting.
#[test]
fn a_window_depth_buffer_is_a_distance_only_after_the_planes_are_applied() {
    let Some((_turn, device, queue)) = device() else {
        return;
    };
    // The two endpoints of the forward formula, asserted before any of this reaches a GPU: `0` is the near
    // plane and `1` is the far one, which is what the specification says those two window-space depths are,
    // and everything the shader does is built on not getting them the other way round.
    for (near, far) in [(0.1, 100.0), (1.5, 500.0), (0.05, 2.0)] {
        assert!(
            window_value(near, far, near).abs() < 1e-6,
            "the near plane is window depth 0, got {}",
            window_value(near, far, near)
        );
        assert!(
            (window_value(near, far, far) - 1.0).abs() < 1e-6,
            "the far plane is window depth 1, got {}",
            window_value(near, far, far)
        );
    }

    // near, far, value, and whether the near triangle at 1 m is standing afterwards.
    let cases: [(f32, f32, f32, bool); 8] = [
        // `0` is the near plane, so these two are a room at `near_z` and nothing else differs.
        (1.5, 100.0, 0.0, true),
        (0.5, 100.0, 0.0, false),
        // And `1` is the far plane, so these two are a room at `far_z`.
        (0.1, 0.8, 1.0, false),
        (0.1, 5.0, 1.0, true),
        // Between the planes the reading is hyperbolic: 1.5 m leaves the 1 m triangle standing...
        (0.1, 100.0, window_value(0.1, 100.0, 1.5), true),
        // ...and 0.9 m takes it, where a *linear* reading of these same two planes would say 88.9 m.
        (0.1, 100.0, window_value(0.1, 100.0, 0.9), false),
        // An infinite frustum, which is what `far < near` means rather than a reversed depth buffer.
        (0.5, 0.0, 0.0, false),
        (0.5, 0.0, 0.9, true),
    ];
    for (near, far, value, expect) in cases {
        let pixel = super::centre_pixel_with(
            &device,
            &queue,
            Depth::ZeroToOne,
            Some(texel(
                &device,
                &queue,
                value,
                wxr::DepthEncoding::Window {
                    near_z: near,
                    far_z: far,
                },
            )),
        );
        assert_eq!(
            standing(pixel),
            expect,
            "a room at window depth {value} over {near}..{far} m: read {pixel:?}, and the 1 m \
             triangle is {}",
            if expect { "standing" } else { "gone" }
        );
    }
}
