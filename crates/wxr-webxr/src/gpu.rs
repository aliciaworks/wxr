//! The WebXR/WebGPU binding, as the browser really spells it.
//!
//! `web-sys` has the WebXR core, `XrRenderStateInit` and the WebGPU objects, and none of the Layers module's
//! `XRProjectionLayer`, nor `XRGPUBinding` or its sub-images. So they are declared here, and **every name in
//! this file was read off a running browser** rather than out of the specification's prose: a name that is
//! wrong here compiles and fails at runtime, and there is no way to find that out but to ask. What the ask
//! answered is in the comments beside each one.
//!
//! What none of this can do without is an XR-compatible device, which is a device made from an adapter
//! requested with `xrCompatible: true` - the field this workspace takes wgpu from a fork for. Given anything
//! else, `XRGPUBinding`'s constructor throws, which is why every call here is caught: a session with the wrong
//! device is a session with no images, not a session that failed.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{XrSession, XrView, XrViewport};

/// The binding a WebGPU-compatible session renders through.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = XRGPUBinding)]
    pub type XrGpuBinding;

    /// `new XRGPUBinding(session, device)`.
    ///
    /// Throws `InvalidStateError` when the session is not WebGPU-compatible, or when the device did not come
    /// from an XR-compatible adapter - so this is the call that cannot be worked around, and the reason the
    /// whole module is written to fail softly.
    #[wasm_bindgen(constructor, catch)]
    pub fn new(session: &XrSession, device: &JsValue) -> Result<XrGpuBinding, JsValue>;

    #[wasm_bindgen(method, catch)]
    pub fn create_projection_layer(
        this: &XrGpuBinding,
        init: &JsValue,
    ) -> Result<XrProjectionLayer, JsValue>;

    /// The sub-image for one view: the same colour texture both eyes get, and the part of it this eye is.
    #[wasm_bindgen(method)]
    pub fn get_view_sub_image(
        this: &XrGpuBinding,
        layer: &XrProjectionLayer,
        view: &XrView,
    ) -> XrGpuSubImage;

    #[wasm_bindgen(method)]
    pub fn get_preferred_color_format(this: &XrGpuBinding) -> String;
}

/// The layer a WebGPU-compatible session presents, which is what it has *instead* of a base layer.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = XRProjectionLayer)]
    pub type XrProjectionLayer;
}

/// One view's worth of a frame's texture.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = XRGPUSubImage)]
    pub type XrGpuSubImage;

    /// The colour texture. Both eyes get the *same* one - the spec says so - which is why a session of this
    /// kind is one image with two views rather than two images, and why this backend reports `image_count` 1.
    #[wasm_bindgen(method, getter)]
    pub fn color_texture(this: &XrGpuSubImage) -> JsValue;

    /// Which part of the texture this view is, in pixels. Inherited from `XRSubImage`, which the browser's
    /// prototype shows and the specification says.
    #[wasm_bindgen(method, getter)]
    pub fn viewport(this: &XrGpuSubImage) -> XrViewport;

    /// The descriptor a texture view has to be made with to draw into this view's part of the texture.
    #[wasm_bindgen(method)]
    pub fn get_view_descriptor(this: &XrGpuSubImage) -> JsValue;
}

/// `XRSession::updateRenderState`, which `web-sys` declares for the base layer and not for layers.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = XRSession)]
    type SessionLayers;

    #[wasm_bindgen(method)]
    fn update_render_state(this: &SessionLayers, state: &JsValue);
}

/// What `createProjectionLayer` takes: a colour format, which is required, and nothing that is this renderer's
/// business to choose. The preferred one comes from the binding, and is what the texture will be.
pub fn projection_layer_init(color_format: &str) -> JsValue {
    let init = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &init,
        &JsValue::from_str("colorFormat"),
        &JsValue::from_str(color_format),
    );
    init.into()
}

/// Whether a session has a feature, which is how the specification says to find out what an `optionalFeatures`
/// request was answered with.
///
/// `web-sys` has no `enabledFeatures`, so this is the reflection it would have generated.
pub fn has_feature(session: &XrSession, feature: &str) -> bool {
    let Ok(value) = js_sys::Reflect::get(session.as_ref(), &JsValue::from_str("enabledFeatures"))
    else {
        return false;
    };
    let Some(features) = value.dyn_ref::<js_sys::Array>() else {
        return false;
    };
    features
        .iter()
        .any(|f| f.as_string().as_deref() == Some(feature))
}

/// Present the layer - which is what a WebGPU-compatible session does *instead* of setting a base layer, and
/// what makes its animation frames start arriving at all.
pub fn set_layers(session: &XrSession, layer: &XrProjectionLayer) {
    let state = js_sys::Object::new();
    let layers = js_sys::Array::of1(layer.as_ref());
    let _ = js_sys::Reflect::set(&state, &JsValue::from_str("layers"), &layers);
    let session: &SessionLayers = session.unchecked_ref();
    session.update_render_state(&state);
}

/// The array layer a sub-image's descriptor starts at: how a stereo projection layer says which eye a view is.
pub fn base_array_layer(descriptor: &JsValue) -> u32 {
    js_sys::Reflect::get(descriptor, &JsValue::from_str("baseArrayLayer"))
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value.max(0.0) as u32)
        .unwrap_or(0)
}

/// What a browser `GPUTexture` says about itself, in the core's terms.
///
/// Read from the texture rather than from the layer's configuration, for the same reason the Apple backend
/// reads it from the texture it is handed: it is what will actually be drawn into. `GPUTexture`'s own getters
/// are synchronous, which is what makes this possible at all.
pub fn image_meta(texture: &JsValue) -> wxr::ImageMeta {
    let field = |name: &str| {
        js_sys::Reflect::get(texture, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_f64())
    };
    let format = js_sys::Reflect::get(texture, &JsValue::from_str("format"))
        .ok()
        .and_then(|value| value.as_string())
        .map(|name| color_format(&name))
        .unwrap_or_default();
    wxr::ImageMeta {
        format,
        extent: wxr::Extent2d::new(
            field("width").unwrap_or(0.0).max(0.0) as u32,
            field("height").unwrap_or(0.0).max(0.0) as u32,
        ),
        layers: field("depthOrArrayLayers").unwrap_or(1.0).max(1.0) as u32,
    }
}

/// A `GPUTextureFormat` in the core's terms. `Unknown` for one this core has not learned, which the renderer
/// turns into a frame it does not draw rather than one drawn in the wrong colour space.
fn color_format(name: &str) -> wxr::ColorFormat {
    match name {
        "bgra8unorm-srgb" => wxr::ColorFormat::Bgra8Srgb,
        "bgra8unorm" => wxr::ColorFormat::Bgra8Unorm,
        "rgba8unorm-srgb" => wxr::ColorFormat::Rgba8Srgb,
        "rgba8unorm" => wxr::ColorFormat::Rgba8Unorm,
        "rgba16float" => wxr::ColorFormat::Rgba16Float,
        "rgb10a2unorm" => wxr::ColorFormat::Rgb10a2Unorm,
        _ => wxr::ColorFormat::Unknown,
    }
}
