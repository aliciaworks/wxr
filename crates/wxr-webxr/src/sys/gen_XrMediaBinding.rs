#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRMediaBinding",
        typescript_type = "XRMediaBinding"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaBinding` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding)"]
    pub type XrMediaBinding;
    #[wasm_bindgen(catch, constructor, js_class = "XRMediaBinding")]
    #[doc = "The `new XrMediaBinding(..)` constructor, creating a new instance of `XrMediaBinding`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/XRMediaBinding)"]
    pub fn new(session: &XrSession) -> Result<XrMediaBinding, JsValue>;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer(
        this: &XrMediaBinding,
        video: &::wasm_bindgen::JsValue,
    ) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createCylinderLayer")]
    #[doc = "The `createCylinderLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createCylinderLayer)"]
    pub fn create_cylinder_layer_with_init(
        this: &XrMediaBinding,
        video: &::wasm_bindgen::JsValue,
        init: &XrMediaCylinderLayerInit,
    ) -> XrCylinderLayer;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createEquirectLayer)"]
    pub fn create_equirect_layer(
        this: &XrMediaBinding,
        video: &::wasm_bindgen::JsValue,
    ) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createEquirectLayer")]
    #[doc = "The `createEquirectLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createEquirectLayer)"]
    pub fn create_equirect_layer_with_init(
        this: &XrMediaBinding,
        video: &::wasm_bindgen::JsValue,
        init: &XrMediaEquirectLayerInit,
    ) -> XrEquirectLayer;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createQuadLayer)"]
    pub fn create_quad_layer(this: &XrMediaBinding, video: &::wasm_bindgen::JsValue)
    -> XrQuadLayer;
    #[wasm_bindgen(method, js_class = "XRMediaBinding", js_name = "createQuadLayer")]
    #[doc = "The `createQuadLayer()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/createQuadLayer)"]
    pub fn create_quad_layer_with_init(
        this: &XrMediaBinding,
        video: &::wasm_bindgen::JsValue,
        init: &XrMediaQuadLayerInit,
    ) -> XrQuadLayer;
}
