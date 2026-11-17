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
//!                        ├─▶ hand tracking: where the hands and their joints are ─────────▶ session::inputs and session::hand
//!                        └─▶ planes and scene reconstruction: the room ──────────────────────▶ session::planes, meshes and hits
//! GameController (ObjC)  ──▶ controllers: buttons and haptics ─────────────────────────────▶ gamecontroller
//! Swift (the app)        ──▶ ImmersiveSpace's CompositorLayer closure + the wgpu device ───▶ entry::run
//! ```

//! **What the platform does not have, said plainly.** `environment_depth` has nothing behind it: there is no
//! depth provider on the visionOS ARKit C surface, and a drawable's depth buffer is the one being drawn into
//! rather than a measurement of the room. The composition layers behind `layer` are the same absence from the
//! other side - CompositorServices gives the app one projection layer, so a quad, cylinder, equirect or cube
//! has nowhere to go, and a layer whose picture the app supplies needs a layer to supply it to. Each answers
//! `None`, an empty list or `Error::Unsupported`, which is exactly what `wxr::Features` exists to say: the
//! capability bits this backend sets are what the platform has, and the ones it does not set are what it does
//! not.
//!
//! **Hit testing is not one of those absences.** The framework has no raycast and no hit-test provider
//! either, but it does have the room - so `ArKit::cast` casts the ray at the triangles scene
//! reconstruction traced and the rectangles plane detection found, and answers with the nearest surface. The
//! kinds a source asks for narrow which of the two are read, so a request that wants only planes never copies
//! the triangles and one that wants no kind at all can never report a hit.
//!
//! **A refusal is not an absence.** Every provider above can be up and answer nothing because the wearer
//! said no, and from a count of providers that is indistinguishable from a device that cannot do it. ARKit
//! keeps the difference in `ar_authorization_status_t`, so `Session::authorization` answers with it and a
//! refused capability stops being counted in `Session::features` - the two questions agree about what this
//! session has, and the app has the sentence to tell the wearer why.
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
