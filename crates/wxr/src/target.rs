//! The picture at the end of the frame: its shape, and where it comes from.
//!
//! There is no image here. [`ImageMeta`] describes the images a session hands out, and the images
//! themselves are [`crate::Session::Image`] - an associated type, because the three platforms do not agree
//! on what one is, and pretending they do is what makes a core that only fits one of them.

/// How many pixels wide and tall.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Extent2d {
    pub width: u32,
    pub height: u32,
}

impl Extent2d {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// How many pixels, which is what a viewport is checked against.
    pub fn pixels(self) -> u32 {
        self.width * self.height
    }
}

/// What a colour channel is made of.
///
/// A short list on purpose: these are the formats a compositor is required to accept, and a runtime that
/// offers something outside it is a runtime to be told what it must offer rather than one to be
/// accommodated. `Unknown` exists so that a backend can report a format this core has not learned yet
/// without failing a frame over it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ColorFormat {
    /// Eight bits each, sRGB-encoded. What every compositor accepts.
    #[default]
    Rgba8Srgb,
    /// Eight bits each, linear. What a compositor that does its own encoding asks for.
    Rgba8Unorm,
    /// Half floats, linear light with values above one. HDR, which is what a headset wants and a tone map
    /// is needed for.
    Rgba16Float,
    /// Ten bits each, packed. HDR at half the bandwidth.
    Rgb10a2Unorm,
    Unknown,
}

impl ColorFormat {
    /// Whether values may exceed one, which changes how the shading has to be written.
    pub fn is_hdr(self) -> bool {
        matches!(self, Self::Rgba16Float | Self::Rgb10a2Unorm)
    }
}

/// The shape of the images a session presents: the same for every image it hands out.
///
/// One format and one extent for all of them, because a compositor presents one thing - the two eyes differ
/// in where they are drawn, not in what they are drawn as.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ImageMeta {
    pub format: ColorFormat,
    pub extent: Extent2d,
    /// Array layers per image: two for a stereo session that puts an eye in a layer, one when the eyes are
    /// two viewports of the same image.
    pub layers: u32,
}

impl ImageMeta {
    /// Whether the eyes are the layers of one image rather than two images.
    pub fn is_layered(self) -> bool {
        self.layers > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_is_what_a_tone_map_is_for() {
        assert!(ColorFormat::Rgba16Float.is_hdr());
        assert!(ColorFormat::Rgb10a2Unorm.is_hdr());
        assert!(!ColorFormat::Rgba8Srgb.is_hdr());
    }

    #[test]
    fn two_layers_is_two_eyes_in_one_image() {
        let meta = ImageMeta {
            layers: 2,
            extent: Extent2d::new(2048, 2048),
            ..Default::default()
        };
        assert!(meta.is_layered());
        assert_eq!(meta.extent.pixels(), 2048 * 2048);
    }
}
