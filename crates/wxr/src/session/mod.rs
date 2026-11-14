//! A runtime, connected and then running.
//!
//! ## Reaching the platform underneath
//!
//! [`Session::as_backend`] is the core's answer to `wgpu`'s `as_hal`, for the same reason: the core says
//! exactly what WebXR says and nothing else, so anything only one platform has is not in the trait, and a
//! program that needs OpenXR's own eight session states, or WebXR's `XRGPUBinding`, or a `cp_drawable`, needs
//! a way back to the type that has them. What comes back is *not* this API - `None` means "a different
//! backend" - so reach for [`Session::visibility`] first and through here only for what the core has decided
//! it will not say. `examples/escape_hatch.rs` is that, against the mock, in a program that runs.

mod api;
mod backend;
mod vocabulary;

pub use api::Session;
pub use backend::Backend;
pub use vocabulary::*;
