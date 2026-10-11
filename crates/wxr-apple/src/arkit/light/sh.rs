//! The projection ARKit's photograph of a room needs to become the core's nine spherical harmonics.
//!
//! Three conversions stand between what ARKit hands over and what [`wxr::LightEstimate`] is, and only one
//! of them is really about Apple, so they are separated here. The room arrives as a **cubemap** - six
//! square faces of radiance - and has to leave as nine numbers, which is a spherical-harmonic projection:
//! geometry, with no API in it. It arrives in **P3** primaries and every renderer in this workspace draws
//! in sRGB, which is a colour conversion: also arithmetic. What is left is the push-to-pull adapter, and
//! that one is Apple's, so it lives in the parent module rather than here.
//!
//! Keeping the arithmetic apart is not tidiness for its own sake. A projection that puts a room's light
//! in the wrong direction does not fail - it lights a wall from the wrong side - and the only way to tell
//! is to feed this a cubemap whose answer is known before the fact. So the caller hands over a *sampler*
//! of radiance and this hands back an estimate, and [`tests`] builds a room out of nothing to check both
//! the coefficients and the direction they came from.
//!
//! **Order two, and nine is as many as order two has.** A spherical harmonic of degree `l` has `2l + 1`
//! coefficients, so degrees zero, one and two are `1 + 3 + 5` - which is the nine the core carries and the
//! nine WebXR asks for. Order two is not a limitation anyone chose here: it is the highest degree that
//! still describes the *ambient* light of a room, which is the light that has already bounced off
//! everything and therefore has no sharp edges left to need more coefficients for.
//!
//! [`tests`]: tests

use wxr::LightEstimate;
use wxr::glam::{Mat3, Vec3};

/// The luminance weights of the sRGB primaries, for ranking texels by how bright they look.
const LUMA: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);

/// The nine real spherical harmonics of degree at most two, at a unit direction, in WebXR's order:
/// `C00`, `C1-1`, `C10`, `C11`, `C2-2`, `C2-1`, `C20`, `C21`, `C22`.
///
/// The constants are the analytic ones - `1 / (2 sqrt(pi))`, `sqrt(3 / (4 pi))` and so on - written out,
/// because a `sqrt` of `PI` is not a constant expression and a table of nine numbers whose derivation is
/// in a comment is easier to check than one whose derivation is somewhere else.
pub(super) fn basis(direction: Vec3) -> [f32; 9] {
    let Vec3 { x, y, z } = direction;
    [
        0.2820948,
        0.4886025 * y,
        0.4886025 * z,
        0.4886025 * x,
        1.0925484 * x * y,
        1.0925484 * y * z,
        0.3153916 * (3.0 * z * z - 1.0),
        1.0925484 * x * z,
        0.5462742 * (x * x - y * y),
    ]
}

/// How ARKit's P3 primaries become the sRGB ones everything drawn here is authored in.
///
/// The two share a white point, so white stays white and only saturated colour moves. P3's primaries are
/// the wider set, so a pure P3 red is *more* than sRGB's red - and has to carry a small negative green
/// and blue to say so. Those negatives are not a mistake and are not clamped away: they are the honest
/// answer, and a scene lit by them is lit by the room's actual colour rather than by a clipped guess at
/// it.
pub(super) fn p3_to_srgb() -> Mat3 {
    // Written to the last digit `f32` can carry and no further, which is what the matrices are to - the
    // colour they describe is a gamut and not a measurement, and eight digits of a gamut is eight more
    // than a display has.
    const P3_TO_XYZ: Mat3 = Mat3::from_cols(
        Vec3::new(0.48657095, 0.22897456, 0.0),
        Vec3::new(0.2656677, 0.69173852, 0.04511338),
        Vec3::new(0.19821729, 0.07928691, 1.0439444),
    );
    const XYZ_TO_SRGB: Mat3 = Mat3::from_cols(
        Vec3::new(3.24097, -0.96924364, 0.05563008),
        Vec3::new(-1.5373832, 1.8759675, -0.20397696),
        Vec3::new(-0.49861076, 0.04155506, 1.0569715),
    );
    XYZ_TO_SRGB * P3_TO_XYZ
}

/// Six faces of radiance as the estimate of the light that made them.
///
/// `size` is the edge of a face and `radiance` answers for one texel - `(face, column, row)` in, and the
/// light coming from that direction in linear sRGB out - which is the whole interface to this function.
/// The projection is a midpoint rule over the sphere, and each texel is weighted by the solid angle it
/// actually covers: a corner texel of a cubemap covers nearly three times what a centre one does, and a
/// projection that forgets that lights every room out of its face centres.
///
/// The primary light is the brightest texel rather than anything the nine coefficients imply, because
/// degree two cannot hold a lamp. A coefficient set is a low-pass filter, and the brightest *direction*
/// of a low-passed room is the middle of a blur; the photograph still has the lamp in it, so that is
/// where the answer comes from.
pub(super) fn project(
    size: usize,
    radiance: impl Fn(usize, usize, usize) -> Vec3,
) -> LightEstimate {
    let mut harmonics = [Vec3::ZERO; 9];
    let mut brightest: Option<(f32, Vec3, Vec3)> = None;
    let texel = 2.0 / size as f32;
    for face in 0..6 {
        for row in 0..size {
            for column in 0..size {
                let u = -1.0 + texel * (column as f32 + 0.5);
                let v = -1.0 + texel * (row as f32 + 0.5);
                let direction = face_direction(face, u, v);
                let light = radiance(face, column, row);
                // The solid angle of one texel of a face at distance one, which falls off from the middle
                // of the face towards its corners as the texel's own tilt away from the viewer grows.
                let solid = texel * texel / (1.0 + u * u + v * v).powf(1.5);
                for (coefficient, weight) in harmonics.iter_mut().zip(basis(direction)) {
                    *coefficient += light * (weight * solid);
                }
                // Positive, and not merely non-negative: a black room has no brightest texel, and the
                // first one it walks past is not a lamp.
                let luminance = light.dot(LUMA);
                if luminance > 0.0 && brightest.is_none_or(|(best, _, _)| luminance > best) {
                    brightest = Some((luminance, direction, light));
                }
            }
        }
    }
    match brightest {
        Some((_, direction, intensity)) => LightEstimate {
            primary_direction: direction,
            primary_intensity: intensity,
            harmonics,
        },
        // A room with no light in it at all is a room with no estimate, which the core says is all zero
        // - a scene lit by nothing rather than by a guess at nothing.
        None => LightEstimate::default(),
    }
}

/// One texel of one face as the direction it is seen from.
///
/// The faces are Metal's own - `+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`, in that slice order - with the texture
/// convention every cubemap on this platform shares, and `u` and `v` run across the face in `[-1, 1]`.
///
/// This is the one part of the conversion that no amount of care can check without a lit room: a sign
/// that is wrong does not make the estimate disappear, it mirrors the room, and the coefficients are
/// still perfectly plausible numbers. It is therefore kept to one small function, with the assumption
/// written down, so that the day somebody does point a headset at a lamp there is exactly one place to
/// look.
fn face_direction(face: usize, u: f32, v: f32) -> Vec3 {
    let direction = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    direction.normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room of one colour: every texel of every face the same light.
    fn solid(size: usize, light: Vec3) -> LightEstimate {
        project(size, |_, _, _| light)
    }

    /// Where a texel of a face points, which the projection's answer has to agree with.
    fn texel_direction(face: usize, size: usize, column: usize, row: usize) -> Vec3 {
        let step = 2.0 / size as f32;
        face_direction(
            face,
            -1.0 + step * (column as f32 + 0.5),
            -1.0 + step * (row as f32 + 0.5),
        )
    }

    /// A room lit the same way everywhere has no direction to it, so every coefficient but the constant
    /// one is zero - and the constant one is the colour times the area of the sphere, which is the
    /// definition of the projection rather than a property of this implementation.
    ///
    /// The constant is out by a part in four thousand, and that is the midpoint rule rather than an error:
    /// `6 * 32^2` rectangles do not tile a sphere exactly.
    #[test]
    fn a_uniform_room_is_only_the_constant_term() {
        let light = Vec3::new(1.0, 0.5, 0.25);
        let estimate = solid(32, light);
        let area = 4.0 * std::f32::consts::PI;
        assert!(
            (estimate.harmonics[0] - light * (0.2820948 * area))
                .abs()
                .max_element()
                < 0.002,
            "C00 was {:?}",
            estimate.harmonics[0]
        );
        for (index, coefficient) in estimate.harmonics.iter().enumerate().skip(1) {
            assert!(
                coefficient.max_element() < 0.000_001,
                "C{index} was {:?}",
                coefficient
            );
        }
    }

    /// One lamp in an otherwise dark room: each coefficient is the lamp's colour times that harmonic at
    /// the lamp's direction times the solid angle of the one texel it occupies. That is the projection of
    /// a delta function, worked out by hand - and it is what pins down the constants, the face directions
    /// and the solid-angle weighting all at once, because a mistake in any of the three moves every
    /// coefficient at once.
    #[test]
    fn a_single_lamp_is_that_lamps_own_coefficients() {
        let size = 32;
        let (face, column, row) = (4, 3, 17);
        let intensity = Vec3::new(3.0, 2.0, 1.0);
        let estimate = project(size, |f, c, r| {
            if (f, c, r) == (face, column, row) {
                intensity
            } else {
                Vec3::ZERO
            }
        });
        let direction = texel_direction(face, size, column, row);
        let step = 2.0 / size as f32;
        let u = -1.0 + step * (column as f32 + 0.5);
        let v = -1.0 + step * (row as f32 + 0.5);
        let solid_angle = step * step / (1.0 + u * u + v * v).powf(1.5);
        for (index, coefficient) in basis(direction).iter().enumerate() {
            let expected = intensity * (*coefficient * solid_angle);
            assert!(
                (estimate.harmonics[index] - expected).abs().max_element() < 0.000_001,
                "C{index} was {:?}, expected {:?}",
                estimate.harmonics[index],
                expected
            );
        }
        let direction = texel_direction(face, size, column, row);
        assert!(
            (estimate.primary_direction - direction).length() < 0.000_001,
            "primary was {:?}, the lamp was at {:?}",
            estimate.primary_direction,
            direction
        );
        assert_eq!(estimate.primary_intensity, intensity);
    }

    /// Twice the light is twice the estimate, which is what makes the coefficients worth carrying: they
    /// are a description of the room's light and not a photograph of a particular exposure of it.
    #[test]
    fn light_scales_the_estimate() {
        let dim = solid(16, Vec3::new(0.25, 0.5, 0.75));
        let bright = solid(16, Vec3::new(0.5, 1.0, 1.5));
        for (dim, bright) in dim.harmonics.iter().zip(bright.harmonics) {
            assert!((bright - *dim * 2.0).abs().max_element() < 0.000_001);
        }
        assert_eq!(bright.primary_intensity, dim.primary_intensity * 2.0);
    }

    /// P3's white is sRGB's white, because the two share a white point and the conversion is only about
    /// where the primaries sit.
    #[test]
    fn white_survives_the_colour_conversion() {
        let white = p3_to_srgb() * Vec3::ONE;
        assert!((white - Vec3::ONE).abs().max_element() < 0.000_01);
    }

    /// P3's red is *outside* sRGB's red, so it arrives as a red over one and a little negative green and
    /// blue - the colour sRGB cannot spell, spelled in the two channels it does have.
    #[test]
    fn p3_red_is_more_than_srgb_red() {
        let red = p3_to_srgb() * Vec3::X;
        assert!(
            (red - Vec3::new(1.224_940_2, -0.042_056_95, -0.019_637_55))
                .abs()
                .max_element()
                < 0.000_01
        );
    }

    /// A dark room is a room with no estimate, not a room whose estimate is a very small one: the core
    /// says all zero means nothing was measured, and a scene has to be able to tell the difference
    /// between a black room and no answer.
    #[test]
    fn a_dark_room_has_no_estimate() {
        assert_eq!(solid(16, Vec3::ZERO), LightEstimate::default());
    }
}
