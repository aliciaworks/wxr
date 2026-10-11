//! ARKit's light estimation as the core's [`wxr::LightEstimate`], and the adapter the two shapes need.
//!
//! The core pulls and ARKit pushes, which is the same mismatch every provider here has, and this one has
//! it worse in a particular way: a plane is a list of anchors and a photograph of a room is six faces of
//! floating-point radiance. So the adapter has three halves and only the first is a shape difference.
//!
//! **The push-to-pull half.** ARKit's provider calls a handler on a queue whenever it has photographed
//! the room again. The handler reads that photograph as far as an estimate and stores it; the frame's
//! [`wxr::Session::light`] is then a mutex and a copy. What it is deliberately *not* is a projection on
//! the frame's thread - blitting six faces off the GPU and waiting for it is the better part of a
//! millisecond, and a wait in a frame is a dropped frame.
//!
//! **The colour half** is in [`sh`], because it is arithmetic. ARKit says plainly that its environment
//! textures are linear HDR in P3, and nothing drawn by this workspace is.
//!
//! **The reading half** is here, because it is Metal. ARKit's texture is a `MTLTextureTypeCube` of `half`
//! or `float`, and the CPU may not simply read a texture the GPU owns: the storage mode decides. Every
//! face is therefore *blitted* to a buffer of this backend's own rather than read where it lies, which is
//! one path that works for a private texture and a shared one, rather than two paths of which one is
//! always the half that was not thought about.
//!
//! **What is missing is missing on purpose.** A room too dark to photograph has no texture, and the core
//! says an estimate of all zero is what a runtime with nothing to say hands over - so that is what a dark
//! room gets, rather than an error to stop a frame over. A texture in a format this has not met is the
//! same answer for the same reason: a guess at four channels is how a room ends up lit by noise.

use std::ffi::c_void;
use std::sync::Mutex;

use dispatch2::{DispatchQueue, DispatchRetained};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_ar_kit::{
    ar_data_providers_t, ar_environment_light_estimation_configuration_t,
    ar_environment_light_estimation_provider_t, ar_environment_probe_anchor_t,
    ar_environment_probe_anchors_t,
};
use objc2_metal_visionos::{
    MTLBlitCommandEncoder, MTLBuffer, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLDevice, MTLOrigin, MTLPixelFormat, MTLResource, MTLResourceOptions, MTLSize, MTLTexture,
    MTLTextureType,
};
use wxr::glam::Vec3;

mod sh;

/// The six faces of a cubemap, which is what one of ARKit's textures is.
const FACES: usize = 6;

/// Light estimation: the provider, and the slot its handler writes estimates into.
pub(super) struct Lights {
    /// Kept for its lifetime, the way the other providers are: releasing it is how a session stops being
    /// told anything.
    _provider: Retained<ar_environment_light_estimation_provider_t>,
    /// Kept because ARKit only holds a queue for as long as it is calling on it.
    _queue: DispatchRetained<DispatchQueue>,
    /// The freshest estimate, and `None` for every frame before ARKit has seen a lit room.
    estimate: &'static Mutex<Option<wxr::LightEstimate>>,
}

impl Lights {
    /// Whether this device can estimate a room's light at all.
    ///
    /// The same question the other providers ask first, for the same reason: a provider on a device that
    /// has none is a provider whose every update never comes.
    pub(super) fn is_supported() -> bool {
        // SAFETY: the question takes no arguments and has no preconditions.
        unsafe { ar_environment_light_estimation_provider_t::is_supported() }
    }

    /// The estimate ARKit has most recently produced, or `None` for a room it has not photographed.
    ///
    /// A copy, like every other read here: the photograph is ARKit's and the estimate is what the frame
    /// gets, not what it keeps.
    pub(super) fn estimate(&self) -> Option<wxr::LightEstimate> {
        // A poisoned slot means something panicked while writing one, and the only writer is a handler
        // that cannot panic - so this is unreachable, and a frame is the wrong place to find that out.
        *self
            .estimate
            .lock()
            .expect("the light slot is only ever written by a handler that cannot panic")
    }

    /// Start the provider and the queue its handler runs on, or `None` if the device cannot.
    ///
    /// # Safety
    ///
    /// `providers` must be a live collection.
    pub(super) unsafe fn start(providers: &ar_data_providers_t) -> Option<Self> {
        // SAFETY: as the other providers - the calls build ARKit objects, and the provider is added to
        // the collection the session retains.
        unsafe {
            if !Self::is_supported() {
                return None;
            }
            let configuration = ar_environment_light_estimation_configuration_t::new();
            let provider = ar_environment_light_estimation_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, super::as_data_provider(&provider));
            // A queue of this backend's own, and not `NULL`: `NULL` means the main queue, which is where
            // an app waits for the compositor - and an estimate that has to queue behind a frame is one
            // that arrives after it was wanted. Serial, because two photographs of one room should not
            // be read in either order.
            let queue = DispatchQueue::new(Some(c"wxr.light"), None);
            // The slot is leaked, and this is the place to say why rather than the place to regret it.
            // ARKit keeps the context pointer for as long as the provider lives, and the order in which
            // it lets go of the handler, the provider and the session is not something this backend can
            // observe from the outside - so there is no point at which the pointer can be shown to be
            // finished with. Forty bytes, once per session, is the cheap end of that trade: the other end
            // is a handler writing into a session that has already been dropped, which is a crash on a
            // device and nowhere else.
            let estimate: &'static Mutex<Option<wxr::LightEstimate>> =
                Box::leak(Box::new(Mutex::new(None)));
            ar_environment_light_estimation_provider_t::set_update_handler_f(
                &provider,
                Some(&queue),
                estimate as *const _ as *mut c_void,
                Some(probe_updated),
            );
            Some(Self {
                _provider: provider,
                _queue: queue,
                estimate,
            })
        }
    }
}

/// ARKit's handler: a probe anchor has a new photograph of the room, which is where an estimate comes from.
///
/// # Safety
///
/// Called by ARKit with the context [`Lights::start`] passed, which is the address of a slot that is never
/// freed, and with two live collections of anchors.
unsafe extern "C-unwind" fn probe_updated(
    context: *mut c_void,
    added: &ar_environment_probe_anchors_t,
    updated: &ar_environment_probe_anchors_t,
    _removed: &ar_environment_probe_anchors_t,
) {
    // SAFETY: the context is the address `start` leaked, which lives as long as the process does.
    let slot = unsafe { &*context.cast::<Mutex<Option<wxr::LightEstimate>>>() };
    let mut freshest = Freshest::default();
    // SAFETY: both collections are live for the call, and the enumerator only reads the anchor ARKit
    // hands it. Added is walked before updated so that an anchor appearing in both is answered by the
    // newer of the two.
    unsafe {
        for anchors in [added, updated] {
            ar_environment_probe_anchors_t::enumerate_anchors_f(
                anchors,
                std::ptr::from_mut(&mut freshest).cast::<c_void>(),
                take_photograph,
            );
        }
    }
    if let Some(estimate) = freshest.estimate {
        *slot
            .lock()
            .expect("the light slot is only ever written by a handler that cannot panic") =
            Some(estimate);
    }
}

/// The freshest photograph ARKit has offered: when it was taken, and what it came to once it was read.
#[derive(Default)]
struct Freshest {
    timestamp: f64,
    estimate: Option<wxr::LightEstimate>,
}

/// The enumerator [`probe_updated`] hands ARKit: one probe anchor, read if it is the newest with a picture.
///
/// # Safety
///
/// `context` must be the `*mut Freshest` the caller passed, and `anchor` a live probe anchor.
unsafe extern "C-unwind" fn take_photograph(
    context: *mut c_void,
    anchor: &ar_environment_probe_anchor_t,
) -> bool {
    // SAFETY: the caller guarantees both.
    let freshest = unsafe { &mut *context.cast::<Freshest>() };
    // SAFETY: the anchor is live, and asking when a photograph was taken consumes nothing.
    let timestamp = unsafe { ar_environment_probe_anchor_t::timestamp(anchor) };
    if timestamp <= freshest.timestamp {
        return true;
    }
    // ARKit hands over no texture for a room it cannot photograph well enough, and says so itself. That
    // is a frame with no estimate rather than a frame that failed, so the old one is left alone here and
    // only the read below is allowed to clear it.
    let Some(texture) = (unsafe { ar_environment_probe_anchor_t::environment_texture(anchor) })
    else {
        return true;
    };
    // SAFETY: as above.
    let scale = unsafe { ar_environment_probe_anchor_t::camera_scale_reference(anchor) };
    freshest.timestamp = timestamp;
    // A photograph this backend cannot read is no estimate, and not the stale one: the room it was taken
    // in has since changed, and lighting a scene by yesterday's room is worse than not lighting it.
    freshest.estimate = estimate(&texture, scale);
    true
}

/// One of ARKit's photographs as an estimate, or nothing for a texture this cannot read.
///
/// `scale` is the anchor's camera scale reference, which is what makes the numbers absolute: the texture
/// stores brightness relative to its own latest update, and the reference is the absolute brightness of a
/// pixel that reads `[1, 1, 1]`. Without it two photographs of the same room a minute apart would be lit
/// at different strengths.
fn estimate(texture: &ProtocolObject<dyn MTLTexture>, scale: f32) -> Option<wxr::LightEstimate> {
    // A cube, and a square one. The projection below is six square faces, so a texture that is anything
    // else is the wrong thing to project rather than a worse thing to project.
    if texture.textureType() != MTLTextureType::TypeCube {
        log::debug!("wxr-apple: environment texture is not a cube");
        return None;
    }
    let size = texture.width();
    if size == 0 || size != texture.height() {
        return None;
    }
    let format = texture.pixelFormat();
    let texel = texel_bytes(format)?;
    let raw = read(texture, size, texel)?;
    let faces = decode(format, &raw, size, texel);
    let p3_to_srgb = sh::p3_to_srgb();
    let brightness = Vec3::splat(scale);
    Some(sh::project(size, |face, column, row| {
        let texel = faces.at(face, column, row);
        p3_to_srgb * Vec3::new(texel[0], texel[1], texel[2]) * brightness
    }))
}

/// How many bytes one texel of this format takes, or nothing for a format this does not read.
///
/// Two float formats, and not the third: `rg11b10float` and `rgb9e5float` are formats for *drawing*, and
/// an HDR photograph of a room is not one. A format this has not met before is no estimate, because the
/// alternative is inventing the missing channels.
fn texel_bytes(format: MTLPixelFormat) -> Option<usize> {
    if format == MTLPixelFormat::RGBA32Float {
        Some(4 * 4)
    } else if format == MTLPixelFormat::RGBA16Float {
        Some(4 * 2)
    } else {
        log::debug!("wxr-apple: environment texture is {format:?}, which this does not read");
        None
    }
}

/// All six faces off the GPU, and into something the CPU may read.
///
/// Blitted rather than read where they lie, and that is on purpose: a shared texture the CPU may read
/// directly and a private one it may not read at all, and one blit is the path that works for both
/// instead of two branches of which one is always the half that was not thought about. It costs a blit
/// and a wait, which is why it happens on ARKit's queue and not on a frame's.
fn read(texture: &ProtocolObject<dyn MTLTexture>, size: usize, texel: usize) -> Option<Vec<u8>> {
    let face = size * size * texel;
    let device = texture.device();
    // Shared, so that on unified memory the CPU's half of the buffer is the GPU's half once the command
    // buffer is finished.
    let buffer =
        device.newBufferWithLength_options(FACES * face, MTLResourceOptions::StorageModeShared)?;
    let queue = device.newCommandQueue()?;
    let command = queue.commandBuffer()?;
    let blit = command.blitCommandEncoder()?;
    let origin = MTLOrigin { x: 0, y: 0, z: 0 };
    let extent = MTLSize {
        width: size,
        height: size,
        depth: 1,
    };
    for slice in 0..FACES {
        // SAFETY: the texture is a cube of this size and outlives the command buffer, and the buffer is
        // `FACES * face` bytes with this slice's `face` bytes of it yet to be filled.
        unsafe {
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
                texture,
                slice,
                0,
                origin,
                extent,
                &buffer,
                slice * face,
                size * texel,
                face,
            );
        }
    }
    blit.endEncoding();
    command.commit();
    command.waitUntilCompleted();
    // `contents` hands back a pointer and promises nothing about it, so what the two statements below
    // have to establish is the one thing it does not: that the memory is this function's to read.
    let contents = buffer.contents();
    if contents.is_null() {
        return None;
    }
    // SAFETY: the buffer is shared, so its contents are the CPU's to read now the command buffer is done,
    // and it is `FACES * face` bytes long - which is the length asked for below.
    Some(unsafe { std::slice::from_raw_parts(contents.cast::<u8>(), FACES * face).to_vec() })
}

/// Six faces of bytes as six faces of RGBA, in the order the projection reads them.
///
/// Only reached for a format [`texel_bytes`] accepted, so the second arm is the `half` one; a texel the
/// read did not produce is a black one, which is the honest answer to a buffer that came out shorter than
/// the projection believes it is.
fn decode(format: MTLPixelFormat, raw: &[u8], size: usize, texel: usize) -> Faces {
    let mut texels = Vec::with_capacity(FACES * size * size);
    for face in 0..FACES {
        for row in 0..size {
            for column in 0..size {
                let start = ((face * size + row) * size + column) * texel;
                let bytes = raw.get(start..start + texel).unwrap_or(&[]);
                texels.push(if format == MTLPixelFormat::RGBA32Float {
                    rgba32(bytes)
                } else {
                    rgba16(bytes)
                });
            }
        }
    }
    Faces { size, texels }
}

/// Six faces of RGBA, which is all the projection knows about where the light came from.
struct Faces {
    size: usize,
    texels: Vec<[f32; 4]>,
}

impl Faces {
    /// One texel: the face, and the column and row across it.
    ///
    /// Black for a texel that is not there. It cannot be - `decode` builds exactly as many as the
    /// projection walks past - but an out-of-bounds index is a panic in a handler that unwinds into
    /// Objective-C, and a black texel is a bug with a survivable answer.
    fn at(&self, face: usize, column: usize, row: usize) -> [f32; 4] {
        let index = (face * self.size + row) * self.size + column;
        self.texels.get(index).copied().unwrap_or([0.0; 4])
    }
}

/// One `rgba32float` texel.
fn rgba32(bytes: &[u8]) -> [f32; 4] {
    let mut texel = [0.0; 4];
    for (channel, word) in texel.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *channel = f32::from_le_bytes(*word);
    }
    texel
}

/// One `rgba16float` texel, which is the usual shape for an HDR photograph.
fn rgba16(bytes: &[u8]) -> [f32; 4] {
    let mut texel = [0.0; 4];
    for (channel, word) in texel.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *channel = half(u16::from_le_bytes(*word));
    }
    texel
}

/// One IEEE 754 half as the float it is, bit by bit.
///
/// Written out rather than reached for as a dependency, because it is fifteen lines and the standard
/// library has no `f16`: one sign bit, five exponent bits with a bias of fifteen, ten mantissa bits with
/// an implicit one that an exponent of zero takes back.
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = ((bits >> 10) & 0x1f) as i32;
    let mantissa = (bits & 0x3ff) as f32;
    match exponent {
        0 => sign * mantissa * (-24.0f32).exp2(),
        31 if mantissa == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + mantissa / 1024.0) * ((exponent - 15) as f32).exp2(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bits a `half` is, from the value - which is the arithmetic above, run backwards, so that the
    /// test does not agree with itself by construction.
    fn bits(value: f32) -> u16 {
        let bits = value.to_bits();
        let sign = ((bits >> 16) & 0x8000) as u16;
        let mantissa = ((bits >> 13) & 0x3ff) as u16;
        let exponent = ((bits >> 23) & 0xff) as i32;
        // A zero float is a zero half, and a float exponent below the half's would shift the other way.
        if exponent == 0 {
            return sign | mantissa;
        }
        sign | (((exponent - 127 + 15) as u16) << 10) | mantissa
    }

    /// The values an environment photograph is made of, and the two ends that would silently ruin it: a
    /// subnormal and an infinity.
    #[test]
    fn halves_are_read_as_floats() {
        for value in [0.0, 1.0, 0.5, 2.0, 0.333_25, 6.103_515_6e-5, 65_504.0] {
            assert!(
                (half(bits(value)) - value).abs() <= value * 0.001,
                "{value} came back as {}",
                half(bits(value))
            );
        }
        assert_eq!(half(0), 0.0);
        assert_eq!(half(0x8000), -0.0);
        assert_eq!(half(0x7c00), f32::INFINITY);
        assert!(half(0x7e00).is_nan());
    }

    /// A row of `rgba16float` texels is four channels of half, and a row of `rgba32float` is four of
    /// float - which is the whole of what the two arms of `decode` do.
    #[test]
    fn texels_are_four_channels() {
        let mut half_bytes = Vec::new();
        for channel in [1.0f32, -2.0, 0.25, 4.0] {
            half_bytes.extend_from_slice(&bits(channel).to_le_bytes());
        }
        assert_eq!(rgba16(&half_bytes), [1.0, -2.0, 0.25, 4.0]);

        let mut float_bytes = Vec::new();
        for channel in [0.5f32, 3.0, 0.0, 1.5] {
            float_bytes.extend_from_slice(&channel.to_le_bytes());
        }
        assert_eq!(rgba32(&float_bytes), [0.5, 3.0, 0.0, 1.5]);
    }

    /// A buffer shorter than the faces it is supposed to hold reads as black rather than as whatever
    /// follows it, which is the promise `Faces::at` makes about a texel that is not there.
    #[test]
    fn a_short_read_is_black_rather_than_rubbish() {
        let faces = decode(MTLPixelFormat::RGBA32Float, &[0u8; 4], 2, 16);
        assert_eq!(faces.at(0, 0, 0), [0.0; 4]);
        assert_eq!(faces.at(5, 1, 1), [0.0; 4]);
    }
}
