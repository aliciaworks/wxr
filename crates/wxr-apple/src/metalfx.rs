//! MetalFX's spatial scaler, where the SDK has it.
//!
//! The scaler is one pass that turns a small picture into a bigger one with more than a filter: two eyes at a
//! compositor's resolution are more pixels than a forward renderer can shade, and this is the one part of the
//! pipeline whose whole purpose is to make fewer pixels look like more.
//!
//! **Two builds, because the framework exists in one of them.** `MetalFX.framework` is in the device SDK and
//! not in the simulator's, and the simulator is where this is developed - so the device build links the typed
//! bindings and the simulator build has a `SpatialScaler` that answers "no" and links nothing. The two have the
//! same surface, so the renderer never knows which one it was built with, and the fallback is the pass it has
//! anyway.
//!
//! **Why the scaler writes its own texture.** The output has to allow what the scaler writes through - shader
//! writes - and the compositor's image is the compositor's. So the full-size picture is made here and copied
//! into the layer afterwards with a blit, which moves pixels and does not shade them.

#[cfg(not(target_abi = "sim"))]
mod device;
#[cfg(target_abi = "sim")]
mod simulator;

#[cfg(not(target_abi = "sim"))]
pub use device::SpatialScaler;
#[cfg(target_abi = "sim")]
pub use simulator::SpatialScaler;

#[cfg(all(test, not(target_abi = "sim")))]
mod tests;
