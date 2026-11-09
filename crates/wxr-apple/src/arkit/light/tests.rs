//! What `light.rs` reads and what it refuses.
use super::*;

/// The bits a `half` is, from the value - which is the arithmetic above, run backwards, so that the
/// test does not agree with itself by construction.
fn bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let mantissa = ((bits >> 13) & 0x3ff) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    // A zero float is a zero half, and a float exponent below the half's would shift the other way.
    if exponent == 0 {
        return sign | mantissa;
    }
    sign | (((exponent - 127 + 15) as u16) << 10) | mantissa
}

/// The values an environment photograph is made of, and the two ends that would silently ruin it: a
/// subnormal and an infinity.
#[test]
fn halves_are_read_as_floats() {
    for value in [0.0, 1.0, 0.5, 2.0, 0.333_25, 6.103_515_6e-5, 65_504.0] {
        assert!(
            (half(bits(value)) - value).abs() <= value * 0.001,
            "{value} came back as {}",
            half(bits(value))
        );
    }
    assert_eq!(half(0), 0.0);
    assert_eq!(half(0x8000), -0.0);
    assert_eq!(half(0x7c00), f32::INFINITY);
    assert!(half(0x7e00).is_nan());
}

/// A row of `rgba16float` texels is four channels of half, and a row of `rgba32float` is four of
/// float - which is the whole of what the two arms of `decode` do.
#[test]
fn texels_are_four_channels() {
    let mut half_bytes = Vec::new();
    for channel in [1.0f32, -2.0, 0.25, 4.0] {
        half_bytes.extend_from_slice(&bits(channel).to_le_bytes());
    }
    assert_eq!(rgba16(&half_bytes), [1.0, -2.0, 0.25, 4.0]);

    let mut float_bytes = Vec::new();
    for channel in [0.5f32, 3.0, 0.0, 1.5] {
        float_bytes.extend_from_slice(&channel.to_le_bytes());
    }
    assert_eq!(rgba32(&float_bytes), [0.5, 3.0, 0.0, 1.5]);
}

/// A pass with no photograph is not news, so it keeps the estimate. A photograph that could not be
/// read is news that the room is no longer known, so it clears it. Leaving the old estimate in that
/// second case is lighting a changed room with yesterday's light.
#[test]
fn an_unreadable_photograph_clears_the_estimate() {
    let mut slot = Seen {
        estimate: Some(wxr::LightEstimate::default()),
        ..Default::default()
    };

    Freshest::default().keep(&mut slot);
    assert!(
        slot.estimate.is_some(),
        "no photograph at all leaves the last estimate alone"
    );

    Freshest {
        estimate: Some(None),
        ..Default::default()
    }
    .keep(&mut slot);
    assert!(
        slot.estimate.is_none(),
        "a photograph that could not be read is no estimate"
    );
}

/// A readable photograph replaces the last one, so the slot is always the freshest estimate and never
/// the first one that happened to arrive.
#[test]
fn a_readable_photograph_replaces_the_estimate() {
    let mut slot = Seen {
        estimate: Some(wxr::LightEstimate::default()),
        ..Default::default()
    };
    let lit = wxr::LightEstimate {
        primary_intensity: Vec3::ONE,
        ..Default::default()
    };

    Freshest {
        timestamp: 10.0,
        estimate: Some(Some(lit)),
    }
    .keep(&mut slot);
    assert_eq!(slot.estimate.unwrap().primary_intensity, Vec3::ONE);
    assert_eq!(slot.timestamp, 10.0);
}

/// A batch that arrives late is not a newer photograph. The baseline is the timestamp already in the
/// slot, so a second pass can only move the estimate forward - not back to an older room.
#[test]
fn a_stale_batch_does_not_replace_a_newer_estimate() {
    let newer = wxr::LightEstimate {
        primary_intensity: Vec3::ONE,
        ..Default::default()
    };
    let older = wxr::LightEstimate {
        primary_intensity: Vec3::ZERO,
        ..Default::default()
    };
    let mut slot = Seen {
        timestamp: 20.0,
        estimate: Some(newer),
    };

    // What `probe_updated` builds for a batch whose photographs are all older than the slot's.
    let freshest = Freshest::after(&slot);
    // `take_photograph` refuses each of them, so the pass reads nothing and `keep` does nothing.
    assert!(!freshest.newer(15.0), "an older photograph is not fresh");
    assert!(!freshest.newer(20.0), "and neither is the same one again");
    assert!(freshest.newer(25.0), "only a newer one is");
    freshest.keep(&mut slot);
    assert_eq!(slot.estimate.unwrap().primary_intensity, Vec3::ONE);
    assert_eq!(slot.timestamp, 20.0);

    // And a pass that did read one can only move it forward.
    Freshest {
        timestamp: 30.0,
        estimate: Some(Some(older)),
    }
    .keep(&mut slot);
    assert_eq!(slot.estimate.unwrap().primary_intensity, Vec3::ZERO);
    assert_eq!(slot.timestamp, 30.0);

    // A pass that began before the newer batch landed finishes after it. It read a photograph, but its
    // timestamp is behind the slot's now, so putting it in would still be a backwards step.
    let lit = wxr::LightEstimate {
        primary_intensity: Vec3::ONE,
        ..Default::default()
    };
    Freshest {
        timestamp: 25.0,
        estimate: Some(Some(lit)),
    }
    .keep(&mut slot);
    assert_eq!(slot.estimate.unwrap().primary_intensity, Vec3::ZERO);
    assert_eq!(slot.timestamp, 30.0);
}

/// A buffer shorter than the faces it is supposed to hold reads as black rather than as whatever
/// follows it, which is the promise `Faces::at` makes about a texel that is not there.
#[test]
fn a_short_read_is_black_rather_than_rubbish() {
    let faces = decode(MTLPixelFormat::RGBA32Float, &[0u8; 4], 2, 16);
    assert_eq!(faces.at(0, 0, 0), [0.0; 4]);
    assert_eq!(faces.at(5, 1, 1), [0.0; 4]);
}
