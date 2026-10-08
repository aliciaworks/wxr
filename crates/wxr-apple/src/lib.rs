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
//! * `cp_view_get_transform` and `cp_drawable_compute_projection` are C functions on `sys`, and Apple's C
//!   guide
//!   [Drawing fully immersive content using Metal](https://developer.apple.com/documentation/compositorservices/drawing-fully-immersive-content-using-metal)
//!   calls them. `objc2-compositor-services` does not bind them, which is why `sys` declares them - not
//!   because they are not there - and because they return a `simd` vector, the return crosses through the
//!   small C shim the app builds (`apps/visionos/wxr_compositor_shim.c`): a stable Rust `extern` cannot
//!   name a vector type, and a struct would read the wrong register. `cp_view_get_tangents` was the third of
//!   these and is no longer used at all: a mixed-reality layer refuses it and wants the projection matrix.
//! * ARKit has a **C API**, built for exactly this case and documented as
//!   [ARKit in visionOS C API](https://developer.apple.com/documentation/arkit/arkit-in-visionos-c-api): a
//!   session, providers, anchors, hands. `arkit` is that session and the two providers this leg asks for.
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
//! ARKit (C)              ──▶ world tracking: where the head is ────────────────────────────▶ arkit
//!                        ├─▶ hand tracking: where the hands are ───────────────────────────▶ session::inputs
//!                        └─▶ planes and scene reconstruction: the room ───────────────────────────▶ session::planes and meshes
//! GameController (ObjC)  ──▶ controllers: buttons and haptics ─────────────────────────────▶ gamecontroller
//! Swift (the app)        ──▶ ImmersiveSpace's CompositorLayer closure + the wgpu device ───▶ entry::run
//! ```

//! **What the platform does not have, said plainly.** Four of the core's optional methods have nothing
//! behind them here, and three of those are not going to. `depth` and `hit_test_source` are not on the
//! visionOS ARKit C surface at all - there is no depth provider and no hit-test provider in the framework -
//! and CompositorServices has one projection layer rather than the composition layers `layer` would need, so
//! quad, cylinder, equirect and cube have nowhere to go - and `binding` is
//! the same absence, because a layer whose picture the app supplies needs a layer to supply it to. `light_probe` is the fourth and a different kind of
//! absence: the API does have an environment probe, but it hands back a *cubemap* rather than the spherical
//! harmonics the core's `LightEstimate` is, so it maps to WebXR's reflection cubemap - which the core has no
//! word for yet. Each answers `None`, an empty list or `Error::Unsupported`, which is exactly what
//! `wxr::Features` exists to say: the capability bits this backend sets are what the platform has, and the
//! ones it does not set are what it does not.
//!
//! **Haptics are a split, and worth naming as one.** `pulse` is filled - a controller is a `GCController`,
//! its act of buzzing is a Core Haptics pattern played on an engine, and `crate::gamecontroller` is where
//! that is - and `play_pcm` is not: Apple's haptics are patterns rather than waveforms, so there is nothing
//! for samples to become. A phone's Taptic Engine and a pad's handles are both asked in the same words, which
//! is what makes one call fillable and the other a shape this platform simply does not have.

#![cfg(target_vendor = "apple")]

pub mod arkit;
pub mod compositor;
pub mod entry;
pub mod ffi;
pub mod import;
pub mod metal;
pub mod session;
pub mod sys;

mod gamecontroller;

pub use arkit::ArKit;
pub use compositor::Compositor;
pub use entry::run;
pub use import::Images;
pub use session::{AppleBackend, AppleSession};
