//! What a platform implements to be an XR runtime: a way in, and the session that comes out.
//!
//! Separate from the module root because the two sides of this seam grow at different rates: what a session
//! *says* gains a method every time a runtime has a new one, while how a session is made is the same
//! question it has always been and is not going to need the room.

use super::{Error, Session, SessionMode};

/// An XR runtime that can be connected to.
pub trait Backend {
    /// What the renderer has to hand over.
    ///
    /// An associated type for the same reason [`Session::Image`] is one: a Vulkan device, a D3D12 device
    /// and nothing at all are not one thing. What it says is *who makes it* - the renderer does, and the
    /// backend is told afterwards. The other order, where the runtime makes a device for the renderer to
    /// adopt, is how an XR layer ends up owning every graphics decision above it.
    type Device;
    type Session: Session;

    /// Connect, which is not the same as running: a session that arrives asynchronously comes back
    /// `Connecting` and is polled like everything else.
    ///
    /// The mode is WebXR's `requestSession` argument - and it is a request: a platform that decides this itself
    /// takes it and answers in its own terms.
    fn connect(&self, device: Self::Device, mode: SessionMode) -> Result<Self::Session, Error>;
}
