#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRGPUBinding",
        typescript_type = "XRGPUBinding"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuBinding` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding)"]
    pub type XrgpuBinding;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRGPUBinding",
        js_name = "nativeProjectionScaleFactor"
    )]
    #[doc = "Getter for the `nativeProjectionScaleFactor` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/nativeProjectionScaleFactor)"]
    pub fn native_projection_scale_factor(this: &XrgpuBinding) -> f64;
    #[wasm_bindgen(method, getter, js_class = "XRGPUBinding", js_name = "usesDepthValues")]
    #[doc = "Getter for the `usesDepthValues` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/usesDepthValues)"]
    pub fn uses_depth_values(this: &XrgpuBinding) -> bool;
    #[wasm_bindgen(catch, constructor, js_class = "XRGPUBinding")]
    #[doc = "The `new XrgpuBinding(..)` constructor, creating a new instance of `XrgpuBinding`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/XRGPUBinding)"]
    pub fn new(
        session: &XrSession,
        device: &::wasm_bindgen::JsValue,
    ) -> Result<XrgpuBinding, JsValue>;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createCubeLayer")]
    #[doc = "The `createCubeLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createCubeLayer)"]
    pub fn create_cube_layer(this: &XrgpuBinding) -> XrCubeLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createCubeLayer")]
    #[doc = "The `createCubeLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createCubeLayer)"]
    pub fn create_cube_layer_with_init(
        this: &XrgpuBinding,
        init: &XrgpuCubeLayerInit,
    ) -> XrCubeLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer(this: &XrgpuBinding) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer_with_init(
        this: &XrgpuBinding,
        init: &XrgpuCylinderLayerInit,
    ) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createEquirectLayer)"]
    pub fn create_equirect_layer(this: &XrgpuBinding) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createEquirectLayer)"]
    pub fn create_equirect_layer_with_init(
        this: &XrgpuBinding,
        init: &XrgpuEquirectLayerInit,
    ) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createProjectionLayer")]
    #[doc = "The `createProjectionLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createProjectionLayer)"]
    pub fn create_projection_layer(this: &XrgpuBinding) -> XrProjectionLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createProjectionLayer")]
    #[doc = "The `createProjectionLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createProjectionLayer)"]
    pub fn create_projection_layer_with_init(
        this: &XrgpuBinding,
        init: &XrgpuProjectionLayerInit,
    ) -> XrProjectionLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createQuadLayer)"]
    pub fn create_quad_layer(this: &XrgpuBinding) -> XrQuadLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/createQuadLayer)"]
    pub fn create_quad_layer_with_init(
        this: &XrgpuBinding,
        init: &XrgpuQuadLayerInit,
    ) -> XrQuadLayer;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "getDepthInformation")]
    #[doc = "The `getDepthInformation()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/getDepthInformation)"]
    pub fn get_depth_information(
        this: &XrgpuBinding,
        view: &XrView,
    ) -> Option<XrgpuDepthInformation>;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "getPreferredColorFormat")]
    #[doc = "The `getPreferredColorFormat()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/getPreferredColorFormat)"]
    pub fn get_preferred_color_format(this: &XrgpuBinding) -> ::alloc::string::String;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "getSubImage")]
    #[doc = "The `getSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/getSubImage)"]
    pub fn get_sub_image(
        this: &XrgpuBinding,
        layer: &XrCompositionLayer,
        frame: &XrFrame,
    ) -> XrgpuSubImage;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "getSubImage")]
    #[doc = "The `getSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/getSubImage)"]
    pub fn get_sub_image_with_eye(
        this: &XrgpuBinding,
        layer: &XrCompositionLayer,
        frame: &XrFrame,
        eye: XrEye,
    ) -> XrgpuSubImage;
    #[wasm_bindgen(method, js_class = "XRGPUBinding", js_name = "getViewSubImage")]
    #[doc = "The `getViewSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUBinding/getViewSubImage)"]
    pub fn get_view_sub_image(
        this: &XrgpuBinding,
        layer: &XrProjectionLayer,
        view: &XrView,
    ) -> XrgpuSubImage;
}
