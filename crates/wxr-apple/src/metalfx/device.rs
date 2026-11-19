//! The scaler itself, for a build that has the framework to link against.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice,
    MTLOrigin, MTLPixelFormat, MTLSize, MTLStorageMode, MTLTexture, MTLTextureDescriptor,
    MTLTextureType, MTLTextureUsage,
};
use objc2_metal_fx::{
    MTLFXSpatialScaler, MTLFXSpatialScalerBase, MTLFXSpatialScalerColorProcessingMode,
    MTLFXSpatialScalerDescriptor,
};

/// One spatial scaler, and the picture it writes.
pub struct SpatialScaler {
    scaler: Retained<ProtocolObject<dyn MTLFXSpatialScaler>>,
    /// The full-size picture the upscale lands in, before the blit into the compositor's own.
    output: Retained<ProtocolObject<dyn MTLTexture>>,
    input: [u32; 2],
    output_size: [u32; 2],
}

impl SpatialScaler {
    /// A scaler from `input` to `output`, for a format it can read and write.
    ///
    /// `None` when the device says it will not scale, which is a fallback and not a failure: the caller has a
    /// pass of its own.
    pub fn new(
        device: &wgpu::Device,
        input: [u32; 2],
        output: [u32; 2],
        format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let format = metal_format(format)?;
        // SAFETY: the device is the one the compositor gave the renderer, and every handle below is its.
        let device = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }?;
        let device = device.raw_device();

        let descriptor = unsafe { MTLFXSpatialScalerDescriptor::new() };
        unsafe {
            descriptor.setColorTextureFormat(format);
            descriptor.setOutputTextureFormat(format);
            descriptor.setInputWidth(input[0] as usize);
            descriptor.setInputHeight(input[1] as usize);
            descriptor.setOutputWidth(output[0] as usize);
            descriptor.setOutputHeight(output[1] as usize);
            // The small picture is already tone mapped and in the compositor's colour space, which is what
            // `perceptual` means: it is treated as sRGB and is not tone mapped a second time.
            descriptor.setColorProcessingMode(MTLFXSpatialScalerColorProcessingMode::Perceptual);
        }
        // SAFETY: the descriptor is configured and the device belongs to this renderer.
        let scaler = unsafe { descriptor.newSpatialScalerWithDevice(device) }?;

        // The scaler writes through a compute kernel, so the target must allow shader writes; the
        // compositor's image is not ours to mark, and private storage is what the property asks for.
        let descriptor = MTLTextureDescriptor::new();
        unsafe {
            descriptor.setTextureType(MTLTextureType::Type2D);
            descriptor.setPixelFormat(format);
            descriptor.setWidth(output[0] as usize);
            descriptor.setHeight(output[1] as usize);
            descriptor.setUsage(
                MTLTextureUsage::ShaderWrite
                    | MTLTextureUsage::ShaderRead
                    | MTLTextureUsage::RenderTarget,
            );
            descriptor.setStorageMode(MTLStorageMode::Private);
        }
        let output_texture = device.newTextureWithDescriptor(&descriptor)?;

        Some(Self {
            scaler,
            output: output_texture,
            input,
            output_size: output,
        })
    }

    /// Scale `source` and copy the result into one layer of `target`, at `origin`.
    ///
    /// The work goes into a command buffer of wgpu's own making and is submitted with it, so it lands after
    /// whatever the renderer has drawn and before anything that comes later - which is the ordering the
    /// compositor's image needs.
    pub fn encode(
        &self,
        queue: &wgpu::Queue,
        source: &wgpu::Texture,
        target: &wgpu::Texture,
        layer: u32,
        origin: [u32; 2],
    ) -> bool {
        // The handles are kept by the runtime rather than by a wgpu guard: `as_hal` holds the device's snatch
        // lock, and taking it twice on the same device is a panic.
        let (Some(source), Some(target)) = (keep(source), keep(target)) else {
            return false;
        };
        unsafe {
            self.scaler.setColorTexture(Some(&source));
            self.scaler.setInputContentWidth(self.input[0] as usize);
            self.scaler.setInputContentHeight(self.input[1] as usize);
            self.scaler.setOutputTexture(Some(&self.output));
        }

        // A command buffer of the queue's own making, not one of wgpu's: wgpu creates its buffers without
        // retained references, and MetalFX's work refers to objects the command buffer has to keep alive.
        // Committed after everything wgpu has submitted on the same queue, which is the ordering the
        // compositor's image needs.
        unsafe {
            let Some(queue) = queue.as_hal::<wgpu::hal::api::Metal>() else {
                return false;
            };
            let Some(command_buffer) = queue.as_raw().commandBuffer() else {
                return false;
            };
            self.scaler.encodeToCommandBuffer(&command_buffer);
            let Some(blit) = command_buffer.blitCommandEncoder() else {
                return false;
            };
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                &self.output,
                0,
                0,
                MTLOrigin { x: 0, y: 0, z: 0 },
                MTLSize {
                    width: self.output_size[0] as usize,
                    height: self.output_size[1] as usize,
                    depth: 1,
                },
                &target,
                layer as usize,
                0,
                MTLOrigin {
                    x: origin[0] as usize,
                    y: origin[1] as usize,
                    z: 0,
                },
            );
            blit.endEncoding();
            command_buffer.commit();
            true
        }
    }
}

/// One of wgpu's textures, kept by the runtime.
fn keep(texture: &wgpu::Texture) -> Option<Retained<ProtocolObject<dyn MTLTexture>>> {
    // SAFETY: the texture belongs to the device this scaler was made on, and the pointer is retained before
    // the guard that lent it is dropped.
    unsafe {
        let texture = texture.as_hal::<wgpu::hal::api::Metal>()?;
        let handle = texture.raw_handle() as *const ProtocolObject<dyn MTLTexture>
            as *mut ProtocolObject<dyn MTLTexture>;
        Retained::retain(handle)
    }
}

/// The formats the scaler can take at both ends.
///
/// It is a short list on purpose: the two a compositor hands out and the two an HDR frame is kept in. A format
/// that is not here is the caller's own pass, which has no opinion about formats.
fn metal_format(format: wgpu::TextureFormat) -> Option<MTLPixelFormat> {
    Some(match format {
        wgpu::TextureFormat::Bgra8UnormSrgb => MTLPixelFormat::BGRA8Unorm_sRGB,
        wgpu::TextureFormat::Rgba8UnormSrgb => MTLPixelFormat::RGBA8Unorm_sRGB,
        wgpu::TextureFormat::Bgra8Unorm => MTLPixelFormat::BGRA8Unorm,
        wgpu::TextureFormat::Rgba8Unorm => MTLPixelFormat::RGBA8Unorm,
        wgpu::TextureFormat::Rgba16Float => MTLPixelFormat::RGBA16Float,
        _ => return None,
    })
}
