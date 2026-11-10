//! Places the runtime keeps fixed in the room, which is WebXR's Anchors module.

use std::fmt;
use std::str::FromStr;

/// A place the runtime is keeping fixed relative to the world, which is WebXR's `XRAnchor`.
///
/// A handle and not a pose, which is the whole point of the module: the runtime re-tracks where the place is as
/// its understanding of the room changes, and what an app holds is the name to ask by. The same shape
/// [`crate::ReferenceSpace`] has, and for the same reason - a caller that cached a pose would be holding a
/// stale one, and a stale anchor is exactly what this exists to avoid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Anchor {
    id: u32,
}

impl Anchor {
    /// An anchor a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}

/// The runtime's own name for an anchor, which is how a place is found again after the app has gone away.
///
/// Sixteen bytes because all three runtimes name one with a UUID: WebXR's `requestPersistentHandle` answers
/// with the string form of it, OpenXR's `XrSpatialAnchorCreateCompleteFB` carries the bytes, and ARKit's
/// `ar_anchor_get_identifier` fills them in. It is *not* [`Anchor::id`]: that is a slot in one session's table
/// and means nothing to the next session, while this one is the runtime's, and is the thing a caller writes
/// down if it wants to ask the same place for the anchor in a later run.
///
/// A backend with no way to persist an anchor answers `None` for [`crate::Session::anchor_id`] rather than
/// inventing a number a caller could save and never find again.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnchorId([u8; 16]);

impl AnchorId {
    /// The runtime's own sixteen bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The bytes, for a caller that is writing one down.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Why a string is not the runtime's name for an anchor.
///
/// The one string any of the three answers with is a UUID, so this is what a text that is not one parses to -
/// and WebXR is the only backend that ever has text to parse. It is a name and not a key: hashing a string that
/// is not a UUID into sixteen bytes would be a name of this core's making, which is exactly the thing the
/// runtime's name is not.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct NotAnAnchorId;

impl fmt::Display for NotAnAnchorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a UUID")
    }
}

impl FromStr for AnchorId {
    type Err = NotAnAnchorId;

    /// A UUID in the form a runtime writes one: thirty-two hex digits, either bare or in the usual
    /// `8-4-4-4-12` groups.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; 16];
        let mut digits = 0usize;
        for character in text.chars() {
            if character == '-' {
                // A hyphen only where one belongs, so `1234-...` is refused rather than read as four digits
                // followed by a separator. The four positions are the ones a UUID is usually grouped into.
                if !matches!(digits, 8 | 12 | 16 | 20) {
                    return Err(NotAnAnchorId);
                }
                continue;
            }
            let value = character.to_digit(16).ok_or(NotAnAnchorId)? as u8;
            if digits >= 32 {
                return Err(NotAnAnchorId);
            }
            bytes[digits / 2] |= if digits % 2 == 0 { value << 4 } else { value };
            digits += 1;
        }
        if digits != 32 {
            return Err(NotAnAnchorId);
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for AnchorId {
    /// The hyphenated lowercase form, which is how a UUID is usually written and how a caller would save it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, byte) in self.0.iter().enumerate() {
            if matches!(index, 4 | 6 | 8 | 10) {
                f.write_str("-")?;
            }
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

// Debug is the display form in brackets, for the same reason a UUID is not much use as decimal bytes.
impl fmt::Debug for AnchorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnchorId({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_grouping_is_a_uuid() {
        let text = "8f14e45f-ceea-467f-a67b-3c9d3b3f2a01";
        let id: AnchorId = text.parse().unwrap();
        assert_eq!(id.as_bytes()[0], 0x8f);
        assert_eq!(id.as_bytes()[15], 0x01);
        assert_eq!(id.to_string(), text);
    }

    #[test]
    fn the_digits_without_the_hyphens_are_the_same_name() {
        let grouped: AnchorId = "8f14e45f-ceea-467f-a67b-3c9d3b3f2a01".parse().unwrap();
        let bare: AnchorId = "8f14e45fceea467fa67b3c9d3b3f2a01".parse().unwrap();
        assert_eq!(grouped, bare);
    }

    #[test]
    fn upper_case_is_the_same_name_as_lower() {
        let lower: AnchorId = "8f14e45f-ceea-467f-a67b-3c9d3b3f2a01".parse().unwrap();
        let upper: AnchorId = "8F14E45F-CEEA-467F-A67B-3C9D3B3F2A01".parse().unwrap();
        assert_eq!(lower, upper);
    }

    #[test]
    fn anything_that_is_not_a_uuid_is_refused() {
        for text in [
            "",
            "8f14e45f-ceea-467f-a67b-3c9d3b3f2a0", // thirty-one digits
            "8f14e45f-ceea-467f-a67b-3c9d3b3f2a011", // thirty-three
            "8f14e45f-ceea-467f-a67b-3c9d3b3f2a0z", // not hex
            "8f14-e45f-ceea-467f-a67b-3c9d3b3f2a0", // a hyphen after four digits
            "8f14e45fce-467f-a67b-3c9d3b3f2a01",   // a hyphen after ten
        ] {
            assert!(
                text.parse::<AnchorId>().is_err(),
                "{text} was read as a UUID"
            );
        }
    }

    #[test]
    fn a_name_round_trips_through_its_text() {
        let bytes = [0xde, 0xad, 0xbe, 0xef, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let id = AnchorId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
        let text = id.to_string();
        assert_eq!(text, "deadbeef-0001-0203-0405-060708090a0b");
        assert_eq!(text.parse::<AnchorId>().unwrap(), id);
    }
}
