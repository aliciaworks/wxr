#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRCubeLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrCubeLayerInit` dictionary."]
    pub type XrCubeLayerInit;
    #[doc = "Get the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, getter = "clearOnAccess")]
    pub fn get_clear_on_access(this: &XrCubeLayerInit) -> Option<bool>;
    #[doc = "Change the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, setter = "clearOnAccess")]
    pub fn set_clear_on_access(this: &XrCubeLayerInit, val: bool);
    #[doc = "Get the `isStatic` field of this object."]
    #[wasm_bindgen(method, getter = "isStatic")]
    pub fn get_is_static(this: &XrCubeLayerInit) -> Option<bool>;
    #[doc = "Change the `isStatic` field of this object."]
    #[wasm_bindgen(method, setter = "isStatic")]
    pub fn set_is_static(this: &XrCubeLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrCubeLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrCubeLayerInit, val: XrLayerLayout);
    #[doc = "Get the `mipLevels` field of this object."]
    #[wasm_bindgen(method, getter = "mipLevels")]
    pub fn get_mip_levels(this: &XrCubeLayerInit) -> Option<u32>;
    #[doc = "Change the `mipLevels` field of this object."]
    #[wasm_bindgen(method, setter = "mipLevels")]
    pub fn set_mip_levels(this: &XrCubeLayerInit, val: u32);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrCubeLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrCubeLayerInit, val: &XrSpace);
    #[doc = "Get the `textureType` field of this object."]
    #[wasm_bindgen(method, getter = "textureType")]
    pub fn get_texture_type(this: &XrCubeLayerInit) -> Option<XrTextureType>;
    #[doc = "Change the `textureType` field of this object."]
    #[wasm_bindgen(method, setter = "textureType")]
    pub fn set_texture_type(this: &XrCubeLayerInit, val: XrTextureType);
    #[doc = "Get the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelHeight")]
    pub fn get_view_pixel_height(this: &XrCubeLayerInit) -> u32;
    #[doc = "Change the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelHeight")]
    pub fn set_view_pixel_height(this: &XrCubeLayerInit, val: u32);
    #[doc = "Get the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelWidth")]
    pub fn get_view_pixel_width(this: &XrCubeLayerInit) -> u32;
    #[doc = "Change the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelWidth")]
    pub fn set_view_pixel_width(this: &XrCubeLayerInit, val: u32);
    #[doc = "Get the `orientation` field of this object."]
    #[wasm_bindgen(method, getter = "orientation")]
    pub fn get_orientation(this: &XrCubeLayerInit) -> ::wasm_bindgen::JsValue;
    #[doc = "Change the `orientation` field of this object."]
    #[wasm_bindgen(method, setter = "orientation")]
    pub fn set_orientation(this: &XrCubeLayerInit, val: &::wasm_bindgen::JsValue);
}
impl XrCubeLayerInit {
    #[doc = "Construct a new `XrCubeLayerInit`."]
    pub fn new(space: &XrSpace, view_pixel_height: u32, view_pixel_width: u32) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_space(space);
        ret.set_view_pixel_height(view_pixel_height);
        ret.set_view_pixel_width(view_pixel_width);
        ret
    }
    #[deprecated = "Use `set_clear_on_access()` instead."]
    pub fn clear_on_access(&mut self, val: bool) -> &mut Self {
        self.set_clear_on_access(val);
        self
    }
    #[deprecated = "Use `set_is_static()` instead."]
    pub fn is_static(&mut self, val: bool) -> &mut Self {
        self.set_is_static(val);
        self
    }
    #[deprecated = "Use `set_layout()` instead."]
    pub fn layout(&mut self, val: XrLayerLayout) -> &mut Self {
        self.set_layout(val);
        self
    }
    #[deprecated = "Use `set_mip_levels()` instead."]
    pub fn mip_levels(&mut self, val: u32) -> &mut Self {
        self.set_mip_levels(val);
        self
    }
    #[deprecated = "Use `set_space()` instead."]
    pub fn space(&mut self, val: &XrSpace) -> &mut Self {
        self.set_space(val);
        self
    }
    #[deprecated = "Use `set_texture_type()` instead."]
    pub fn texture_type(&mut self, val: XrTextureType) -> &mut Self {
        self.set_texture_type(val);
        self
    }
    #[deprecated = "Use `set_view_pixel_height()` instead."]
    pub fn view_pixel_height(&mut self, val: u32) -> &mut Self {
        self.set_view_pixel_height(val);
        self
    }
    #[deprecated = "Use `set_view_pixel_width()` instead."]
    pub fn view_pixel_width(&mut self, val: u32) -> &mut Self {
        self.set_view_pixel_width(val);
        self
    }
    #[deprecated = "Use `set_orientation()` instead."]
    pub fn orientation(&mut self, val: Option<&::wasm_bindgen::JsValue>) -> &mut Self {
        self.set_orientation(val.unwrap_or(&::wasm_bindgen::JsValue::NULL));
        self
    }
}
