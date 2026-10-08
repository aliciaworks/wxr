//! The pictures a session presents, beyond the one the world is drawn into.
//!
//! A session draws the world into images it is handed, and that is most of what XR is. What the platforms also
//! have - where they have it - is a way to hand the runtime a *picture* instead: a rectangle, a curved
//! rectangle, a sphere's worth, a cube, which the compositor places and warps for the optics itself. The app
//! draws it once rather than once per eye, and the compositor is free to show it on its own clock.
//!
//! WebXR calls these `XRCompositionLayer`s and OpenXR `XrCompositionLayer`s. The projection layer - the one the
//! world is drawn into - is one of them on both platforms, and it is deliberately *not* one of these: it is the
//! session's own, and the frame's views are what draw into it. What is left is the other shapes, and the shapes
//! are the vocabulary both platforms share. A session that has them says so with [`crate::Features::LAYER_QUAD`]
//! and its siblings, one bit per shape, because that is exactly how the platforms say it: OpenXR's quad is the
//! core specification and its cylinder, equirect and cube are extensions, and WebXR's are shapes an app asks a
//! session for by name.

use crate::frame::Viewport;
use crate::target::ImageMeta;

/// A shape a layer's picture can be, and the geometry of it.
///
/// The parameters are the ones both platforms agree on, under the names the platforms use: WebXR's
/// `XRQuadLayerInit` and OpenXR's `XrCompositionLayerQuad` want the same width and height in metres; a cylinder
/// wants a radius, the angle it opens through, and the aspect of what is drawn on it on both; an equirect wants
/// a radius and how far it opens horizontally and above and below the horizon. Sizes are metres and angles are
/// radians.
///
/// What is *not* here is anything about how a layer meets the others: which one is on top, whether it is blended
/// with what is behind it, or which eyes see it. Those are the compositor's rules on every platform that has
/// layers, and a renderer that needs to know them is a renderer drawing the composite itself - which is the
/// thing a layer exists to avoid.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LayerShape {
    /// A flat rectangle, facing the space it was made in: a menu, a monitor, a video - anything that is a
    /// screen.
    Quad { width: f32, height: f32 },
    /// A rectangle bent around a vertical cylinder, which is the same content at the distance an arm likes.
    Cylinder {
        radius: f32,
        central_angle: f32,
        aspect: f32,
    },
    /// A sphere's worth of picture, which is what 360° content and a skybox are.
    ///
    /// Two vertical openings rather than one total: both platforms describe how far the picture reaches above
    /// and below the horizon, because a skybox that stops at the horizon and one that covers the zenith are
    /// different pictures that can have the same total height.
    Equirect {
        radius: f32,
        central_horizontal: f32,
        upper_vertical: f32,
        lower_vertical: f32,
    },
    /// Six faces of a cube, in one image - a skybox the compositor can turn without resampling it.
    Cube,
}

impl LayerShape {
    /// What the shape is called, for a message and for a [`crate::Features`] bit to be about.
    pub fn name(self) -> &'static str {
        match self {
            Self::Quad { .. } => "quad",
            Self::Cylinder { .. } => "cylinder",
            Self::Equirect { .. } => "equirect",
            Self::Cube => "cube",
        }
    }
}

/// A picture the compositor places, named rather than carried.
///
/// A handle and not the shape, the same way [`crate::ReferenceSpace`] is a name and not a transform: what a
/// caller holds is the runtime's name for the layer, and the geometry it asked for is the caller's own
/// business. There is a second reason the shape is not in here, and it is the same reason a space's kind is:
/// a layer is made asynchronously on every platform that has them, so what a caller has for a frame or two is
/// a name for something that is not there yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Layer {
    id: u32,
}

impl Layer {
    /// A layer a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}

/// What a layer's picture is, and where in it the app draws.
///
/// The image itself is [`crate::Session::Image`], and it is the same kind of thing a frame's is: a compositor's
/// image is a compositor's image whether the app is drawing a world into it or a menu. What is beside it here is
/// everything else a render target needs to be built from it - and it comes back per frame, because on every
/// platform that has layers the runtime hands out the picture the compositor is about to read, not one the app
/// keeps.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct LayerImage {
    /// The format and extent of the image, which is what decides the render target's own.
    pub meta: ImageMeta,
    /// The part of the image that belongs to this layer. All of it on a platform that gives a layer its own
    /// texture, which every platform here does; a sub-rectangle on one that pools several layers into one.
    pub viewport: Viewport,
}

impl LayerImage {
    /// Whether the viewport is the whole image, which is the common case and the one a renderer can skip
    /// looking at.
    pub fn is_whole(self) -> bool {
        self.viewport.x == 0
            && self.viewport.y == 0
            && self.viewport.width == self.meta.extent.width
            && self.viewport.height == self.meta.extent.height
    }
}

/// How a layer's picture is arranged for the eyes, which is WebXR's `XRLayerLayout`.
///
/// `Mono` is one picture for both eyes, `Stereo` has the two and lets the runtime find them, and the two named
/// ones are the packings - side by side and over and under. `Default` is the runtime choosing, which is what a
/// session answers when the app did not ask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LayerLayout {
    #[default]
    Default,
    Mono,
    Stereo,
    StereoLeftRight,
    StereoTopBottom,
}

/// What a layer's picture is optimized for, which is WebXR's `XRLayerQuality`.
///
/// Neither is a promise and both are hints about where the sampling should go: a text layer wants sharpness
/// where the eye is, a graphics layer wants the whole field even. `Default` is the runtime choosing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LayerQuality {
    #[default]
    Default,
    TextOptimized,
    GraphicsOptimized,
}

/// What a layer's image is, which is WebXR's `XRTextureType`.
///
/// `Texture` is a single image and `TextureArray` is an array with one slice per eye - which is how a stereo
/// layer carries two pictures without packing them side by side.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextureType {
    #[default]
    Texture,
    TextureArray,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layer_is_its_name() {
        // A layer is the runtime's name for one picture, so two handles with the same id are the same layer -
        // and a backend hands out ids that are unique across every shape rather than unique within one.
        assert_eq!(Layer::new(7), Layer::new(7));
        assert_ne!(Layer::new(7), Layer::new(8));
    }

    #[test]
    fn a_shape_is_named_the_way_the_platforms_name_it() {
        assert_eq!(LayerShape::Cube.name(), "cube");
        assert_eq!(
            LayerShape::Cylinder {
                radius: 2.0,
                central_angle: 1.0,
                aspect: 1.5,
            }
            .name(),
            "cylinder"
        );
    }

    #[test]
    fn a_layer_image_knows_when_it_is_the_whole_image() {
        let meta = ImageMeta {
            extent: crate::Extent2d::new(1024, 768),
            ..Default::default()
        };
        let whole = LayerImage {
            meta,
            viewport: Viewport {
                x: 0,
                y: 0,
                width: 1024,
                height: 768,
            },
        };
        assert!(whole.is_whole());
        // One pixel short is not the whole image, and a viewport that starts inside it is not either.
        assert!(
            !LayerImage {
                viewport: Viewport {
                    width: 1023,
                    ..whole.viewport
                },
                ..whole
            }
            .is_whole()
        );
        assert!(
            !LayerImage {
                viewport: Viewport {
                    x: 1,
                    ..whole.viewport
                },
                ..whole
            }
            .is_whole()
        );
    }
}
