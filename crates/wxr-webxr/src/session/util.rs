use super::*;

/// The WebGPU name of the depth format this workspace draws with.
///
/// Named here because this is where it is asked for: a projection layer made without a depth format has no depth
/// texture at all, and one made with another would hand over depth the renderer's pipeline cannot be attached to.
pub(super) const DEPTH_FORMAT_NAME: &str = "depth32float";

/// Whether a session was granted a feature, which is what `enabledFeatures` answers.
///
/// That attribute is generated now, so this is a translation rather than a read by name: a feature is the same
/// string in the request and in the answer, which is WebXR's own spelling of it.
pub(super) fn has_feature(session: &XrSession, feature: &str) -> bool {
    session
        .enabled_features()
        .iter()
        .any(|granted| granted.as_string().as_deref() == Some(feature))
}

/// The array layer a sub-image's descriptor starts at: how a stereo projection layer says which eye a view is.
pub(super) fn base_array_layer(descriptor: &JsValue) -> u32 {
    js_sys::Reflect::get(descriptor, &JsValue::from_str("baseArrayLayer"))
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value.max(0.0) as u32)
        .unwrap_or(0)
}

/// What a browser `GPUTexture` says about itself, in the core's terms.
///
/// Read from the texture rather than from the layer's configuration, for the same reason the Apple backend reads
/// it from the texture it is handed: it is what will actually be drawn into. A `GPUTexture` is deliberately not
/// a generated type - it belongs to wgpu and to the browser - so this is the one place the two meet.
pub(super) fn image_meta(texture: &JsValue) -> wxr::ImageMeta {
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
pub(super) fn color_format(name: &str) -> wxr::ColorFormat {
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
