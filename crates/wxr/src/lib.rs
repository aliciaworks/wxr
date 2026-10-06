//! An XR core: what a session is, and nothing about who provides one.
//!
//! A session has a head pose, a pair of eyes with a field of view each, a place in the world to measure
//! them from, and a picture at the end that somebody presents. That is the whole vocabulary, and it is
//! the vocabulary of **WebXR**, which is the only one of the three platforms that is a specification
//! rather than a vendor's API - OpenXR and RealityKit both describe a superset in their own terms, and a
//! core shaped like the smallest of them is a core the other two can be reduced to.
//!
//! Three things this core deliberately does not know:
//!
//! * **What a graphics API is.** [`Session::Image`] is an associated type. OpenXR's images are Vulkan or
//!   D3D12 handles, WebXR's are objects that only exist once they are bound to a device, and RealityKit
//!   has none at all. Naming any of those here would make the other two second-class, so the core carries
//!   the images and never looks inside them.
//! * **Who creates the device.** The renderer does, and the backend is told about it afterwards. The
//!   other way round - the runtime handing out a device for the renderer to adopt - is how an XR layer
//!   ends up dictating the graphics API, and it is the mistake this core is written against.
//! * **That the app renders at all.** [`Presentation::Composited`] is OpenXR and WebXR, where the app
//!   draws into the compositor's images. [`Presentation::Scene`] is RealityKit, where the platform draws
//!   the scene the app describes and there is no image to draw into. A core that assumes the first cannot
//!   express the third, and the third is not an edge case - it is a whole platform.
//!
//! What is left is small enough to test without a headset, which is what [`mock`] is for.

// The math the core speaks, so a backend does not have to guess which `glam` it means.
pub use glam;

pub mod frame;
pub mod input;
pub mod mock;
pub mod session;
pub mod space;
pub mod target;

pub use frame::{Eye, FieldOfView, Frame, FrameState, View, Viewport};
pub use input::{Axes, Buttons, Handedness, InputId, InputSource, TargetRayMode};
pub use session::{Backend, Blend, Error, Event, Presentation, Session, State, Visibility};
pub use space::{Pose, ReferenceSpace, SpaceKind};
pub use target::{ColorFormat, Extent2d, ImageMeta};
