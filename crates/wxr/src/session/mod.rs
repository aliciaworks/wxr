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
//!
//! It is the one method here that wants a sized session, and the reason is the language's rather than the
//! core's: a generic method is not in a trait object's table. An erased session is a session of one platform
//! anyway - [`Session::Image`] and [`Session::EnvironmentDepth`] already name its types - so a program holding
//! one knows which backend it has, and every question it can ask without that is one [`Session`] answers on the
//! object itself. [`should_render`] is the other half: a question any session can
//! answer, and it takes one unsized.

mod api;
mod backend;
mod vocabulary;

pub use api::Session;
pub use backend::Backend;
pub use vocabulary::*;
