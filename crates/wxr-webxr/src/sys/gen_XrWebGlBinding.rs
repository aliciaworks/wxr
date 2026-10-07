#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRWebGLBinding",
        typescript_type = "XRWebGLBinding"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrWebGlBinding` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding)"]
    pub type XrWebGlBinding;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLBinding",
        js_name = "nativeProjectionScaleFactor"
    )]
    #[doc = "Getter for the `nativeProjectionScaleFactor` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/nativeProjectionScaleFactor)"]
    pub fn native_projection_scale_factor(this: &XrWebGlBinding) -> f64;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLBinding",
        js_name = "usesDepthValues"
    )]
    #[doc = "Getter for the `usesDepthValues` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/usesDepthValues)"]
    pub fn uses_depth_values(this: &XrWebGlBinding) -> bool;
    #[wasm_bindgen(catch, constructor, js_class = "XRWebGLBinding")]
    #[doc = "The `new XrWebGlBinding(..)` constructor, creating a new instance of `XrWebGlBinding`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/XRWebGLBinding)"]
    pub fn new(
        session: &XrSession,
        context: &::wasm_bindgen::JsValue,
    ) -> Result<XrWebGlBinding, JsValue>;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createCubeLayer")]
    #[doc = "The `createCubeLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createCubeLayer)"]
    pub fn create_cube_layer(this: &XrWebGlBinding) -> XrCubeLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createCubeLayer")]
    #[doc = "The `createCubeLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createCubeLayer)"]
    pub fn create_cube_layer_with_init(
        this: &XrWebGlBinding,
        init: &XrCubeLayerInit,
    ) -> XrCubeLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer(this: &XrWebGlBinding) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer_with_init(
        this: &XrWebGlBinding,
        init: &XrCylinderLayerInit,
    ) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createEquirectLayer)"]
    pub fn create_equirect_layer(this: &XrWebGlBinding) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createEquirectLayer)"]
    pub fn create_equirect_layer_with_init(
        this: &XrWebGlBinding,
        init: &XrEquirectLayerInit,
    ) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createProjectionLayer")]
    #[doc = "The `createProjectionLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createProjectionLayer)"]
    pub fn create_projection_layer(this: &XrWebGlBinding) -> XrProjectionLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createProjectionLayer")]
    #[doc = "The `createProjectionLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createProjectionLayer)"]
    pub fn create_projection_layer_with_init(
        this: &XrWebGlBinding,
        init: &XrProjectionLayerInit,
    ) -> XrProjectionLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createQuadLayer)"]
    pub fn create_quad_layer(this: &XrWebGlBinding) -> XrQuadLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/createQuadLayer)"]
    pub fn create_quad_layer_with_init(
        this: &XrWebGlBinding,
        init: &XrQuadLayerInit,
    ) -> XrQuadLayer;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "getDepthInformation")]
    #[doc = "The `getDepthInformation()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/getDepthInformation)"]
    pub fn get_depth_information(
        this: &XrWebGlBinding,
        view: &XrView,
    ) -> Option<XrWebGlDepthInformation>;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "getReflectionCubeMap")]
    #[doc = "The `getReflectionCubeMap()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/getReflectionCubeMap)"]
    pub fn get_reflection_cube_map(
        this: &XrWebGlBinding,
        light_probe: &XrLightProbe,
    ) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "getSubImage")]
    #[doc = "The `getSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/getSubImage)"]
    pub fn get_sub_image(
        this: &XrWebGlBinding,
        layer: &XrCompositionLayer,
        frame: &XrFrame,
    ) -> XrWebGlSubImage;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "getSubImage")]
    #[doc = "The `getSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/getSubImage)"]
    pub fn get_sub_image_with_eye(
        this: &XrWebGlBinding,
        layer: &XrCompositionLayer,
        frame: &XrFrame,
        eye: XrEye,
    ) -> XrWebGlSubImage;
    #[wasm_bindgen(method, js_class = "XRWebGLBinding", js_name = "getViewSubImage")]
    #[doc = "The `getViewSubImage()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLBinding/getViewSubImage)"]
    pub fn get_view_sub_image(
        this: &XrWebGlBinding,
        layer: &XrProjectionLayer,
        view: &XrView,
    ) -> XrWebGlSubImage;
}
