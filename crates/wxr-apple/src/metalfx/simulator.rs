//! The same surface, on a build whose SDK has no MetalFX.
//!
//! The simulator has neither the framework nor a GPU that would run it, so this is not a capability check
//! that happens to fail - there is nothing to link and nothing to ask. The renderer keeps its own pass, which
//! is what it would use on any device that answered no.

/// Never a scaler: the simulator's Metal has none.
pub struct SpatialScaler;

impl SpatialScaler {
    pub fn new(
        _device: &wgpu::Device,
        _input: [u32; 2],
        _output: [u32; 2],
        _format: wgpu::TextureFormat,
    ) -> Option<Self> {
        None
    }

    pub fn encode(
        &self,
        _queue: &wgpu::Queue,
        _source: &wgpu::Texture,
        _target: &wgpu::Texture,
        _layer: u32,
        _origin: [u32; 2],
    ) -> bool {
        false
    }
}
