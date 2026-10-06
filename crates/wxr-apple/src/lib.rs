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
//! **How much of the compositor is C is worth being exact about, because it decides how much Swift there
//! is.** `cp_layer_renderer_query_next_frame` and the frame's lifecycle, `cp_frame`, the drawable and its
//! `cp_drawable::color_texture` textures, and the per-view `cp_view_texture_map` - texture index, slice
//! index, viewport - are all C, and `objc2-compositor-services` binds them. What is *not* C is the thing a
//! renderer most wants: `Drawable.View.transform` and `.tangents` are Swift properties, and Apple's
//! `cp_view_t` page documents them as such - the texture map section of that page lists a C type and the
//! transformations section lists no C function at all. So the compositor's C surface gives the images, the
//! viewports and the timing, and a shim in Swift has to hand over where the eyes are. `session` is that
//! seam written down: `session::EyeView` is what the shim fills, and `AppleSession::set_eyes` is how it
//! arrives.
//!
//! **ARKit is a Swift API and cannot be called from Objective-C.** Its session on visionOS is
//! [`final class ARKitSession`](https://developer.apple.com/documentation/arkit/arkitsession), whose symbol
//! Apple publishes mangled - `s:5ARKit0A7SessionC` - and whose methods are
//! `func run([any DataProvider]) async throws`. An existential and an `async` are two things the
//! Objective-C runtime has no calling convention for, so `objc2` cannot reach it and no amount of
//! `extern_class!` will help. It is the same shim's other job, and the tracking half is what it is for:
//! hands, planes and the room, none of which the compositor knows.
//!
//! So the order is: **present and draw first, track second.** A session today presents, draws into the
//! compositor's textures, and reports the eyes it is told about, in the one space a compositor knows on its
//! own. A floor, a pair of hands and a room arrive with the tracking half, behind the same shim.
//!
//! ```text
//! CompositorServices (C)  ──▶ images, viewports, timing, the frame loop ──▶ session
//! Swift shim              ──▶ per-eye transform + tangents ───────────────▶ session::EyeView
//!                         └─▶ ARKit: hands, planes, the room ─────────────▶ later
//!                              (all three meet the renderer in metal::texture)
//! ```

#![cfg(target_vendor = "apple")]

pub mod import;
pub mod metal;
pub mod session;
