//! The Apple backend for wxr.
//!
//! Which Apple API this is built on is a question with an evidence-based answer, and the evidence is in
//! Apple's own documentation.
//!
//! **RealityKit is not needed.** visionOS presents immersive content two ways: RealityKit draws it for you,
//! or [`CompositorServices`](https://developer.apple.com/documentation/CompositorServices) hands you the
//! Metal textures and per-eye view information and you draw it. CompositorServices is this backend's, for
//! the reason the whole workspace exists: RealityKit drawing means *our* renderer is not drawing on that
//! platform - it is the core's [`wxr::Presentation::Scene`] arm, and it abandons wgpu. CompositorServices is
//! [`wxr::Presentation::Composited`] with a Metal texture per eye, which is the same shape as OpenXR and
//! WebXR and is drawn by the same [`wxr_render`] renderer. The `Scene` arm stays in the core because it is a
//! true property of the platform and a leg somebody will want; it is just not this one.
//!
//! **ARKit is a Swift API and cannot be called from Objective-C.** Its session on visionOS is
//! [`final class ARKitSession`](https://developer.apple.com/documentation/arkit/arkitsession), whose symbol
//! Apple publishes mangled - `s:5ARKit0A7SessionC` - and whose methods are
//! `func run([any DataProvider]) async throws`. An existential and an `async` are two things the
//! Objective-C runtime has no calling convention for, so `objc2` cannot reach it and no amount of
//! `extern_class!` will help. Reaching it means a Swift shim; what that shim is for arrives at the bottom of
//! this comment.
//!
//! **CompositorServices is a C and Objective-C framework.** `cp_layer_renderer_*` are plain C functions and
//! the layer types are objects, so this half is reachable from Rust with nothing but `extern "C"` and
//! `objc2` - no Swift, no build step, no bridge.
//!
//! So the order is: **present and draw first, track second.** The compositor already gives per-eye view
//! information, so the head pose comes from it and ARKit is only needed for hands, planes and the room -
//! and that half can wait for the Swift shim it requires. [`metal`] is the first piece of the drawing half:
//! a compositor's texture as a wgpu one.
//!
//! ```text
//! CompositorServices ──▶ per-eye view info ──▶ wxr::View      (the head, no ARKit)
//!                    └─▶ per-eye MTLTexture ──▶ wgpu::Texture (metal::texture)
//! ARKit (Swift shim) ──▶ hands, planes, the room             (later)
//! ```
#![cfg(target_vendor = "apple")]

pub mod metal;
