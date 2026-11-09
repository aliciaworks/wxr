#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRMesh",
        typescript_type = "XRMesh"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMesh` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh)"]
    pub type XrMesh;
    #[wasm_bindgen(method, getter, js_class = "XRMesh", js_name = "meshSpace")]
    #[doc = "Getter for the `meshSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh/meshSpace)"]
    pub fn mesh_space(this: &XrMesh) -> XrSpace;
    #[wasm_bindgen(method, getter, js_class = "XRMesh", js_name = "vertices")]
    #[doc = "Getter for the `vertices` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh/vertices)"]
    pub fn vertices(this: &XrMesh) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(method, getter, js_class = "XRMesh", js_name = "indices")]
    #[doc = "Getter for the `indices` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh/indices)"]
    pub fn indices(this: &XrMesh) -> ::alloc::vec::Vec<u32>;
    #[wasm_bindgen(method, getter, js_class = "XRMesh", js_name = "lastChangedTime")]
    #[doc = "Getter for the `lastChangedTime` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh/lastChangedTime)"]
    pub fn last_changed_time(this: &XrMesh) -> f64;
    #[wasm_bindgen(method, getter, js_class = "XRMesh", js_name = "semanticLabel")]
    #[doc = "Getter for the `semanticLabel` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMesh/semanticLabel)"]
    pub fn semantic_label(this: &XrMesh) -> Option<::alloc::string::String>;
}
