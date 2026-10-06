//! The Apple backend for wxr.
//!
//! Which Apple API this is built on is a question with an evidence-based answer, and the evidence is in
//! Apple's own documentation.
//!
//! **RealityKit is not needed.** visionOS presents immersive content two ways: RealityKit draws it for you,
//! or [`CompositorServices`](https://developer.apple.com/documentation/CompositorServices) hands you the
//! Metal textures and the per-eye view information and you draw it. CompositorServices is this backend's,
//! for the reason the whole workspace exists: RealityKit drawing means *our* renderer is not drawing on
//! that platform - it is the core's `wxr::Presentation::Scene` arm, and it abandons wgpu.
//! CompositorServices is `wxr::Presentation::Composited` with a Metal texture per eye, which is the same
//! shape as OpenXR and WebXR and is drawn by the same `wxr-render` renderer. The `Scene` arm stays in the
//! core because it is a true property of the platform and a leg somebody will want; it is just not this one.
//!
//! **There is no Swift shim, and there was going to be one.** ARKit's visionOS Swift API - `ARKitSession`,
//! `WorldTrackingProvider` - is Swift to the bone, with mangled symbols and `async` methods taking
//! existentials, and a first pass at this crate concluded from that that the eye transforms must be Swift
//! too and that a shim was needed to cross. Both halves of that were wrong, and Apple's own documentation
//! says so:
//!
//! * `cp_view_get_transform` and `cp_view_get_tangents` are C functions on `sys`, and Apple's C guide
//!   [Drawing fully immersive content using Metal](https://developer.apple.com/documentation/compositorservices/drawing-fully-immersive-content-using-metal)
//!   calls them. `objc2-compositor-services` does not bind them, which is why `sys` declares them - not
//!   because they are not there.
//! * ARKit has a **C API**, built for exactly this case and documented as
//!   [ARKit in visionOS C API](https://developer.apple.com/documentation/arkit/arkit-in-visionos-c-api): a
//!   session, providers, anchors, hands. `tracking` is its world tracking.
//!
//! What is left for Swift is the app's entry: an `ImmersiveSpace` whose `CompositorLayer` closure hands the
//! layer renderer to `AppleBackend::new`. That is the app's three lines, not a bridge this backend needs,
//! and nothing crosses it but a pointer.
//!
//! **The present is closed too**, by giving the presentation event a command buffer of its own on the queue
//! the renderer draws with - see `session`'s module comment, and note that this is what a C-only Compositor
//! Services renderer does as well, because the event has to be committed by somebody.
//!
//! One platform fact belongs to whoever builds the renderer: **visionOS's drawable depth is reverse-Z**, so
//! a pass drawn into one uses `wxr_render::Depth::Reverse`.
//!
//! ```text
//! CompositorServices (C) ──▶ frames, textures, viewports, per-eye transform and tangents ──▶ session
//! ARKit (C)              ──▶ world tracking: where the head is ────────────────────────────▶ tracking
//!                        └─▶ hands, planes, the room ──────────────────────────────────────▶ later
//! Swift (the app)        ──▶ ImmersiveSpace's CompositorLayer closure ─────────────────────▶ AppleBackend
//! ```

#![cfg(target_vendor = "apple")]

pub mod import;
pub mod metal;
pub mod session;
pub mod sys;
pub mod tracking;

pub use session::{AppleBackend, AppleSession};
pub use tracking::WorldTracking;
