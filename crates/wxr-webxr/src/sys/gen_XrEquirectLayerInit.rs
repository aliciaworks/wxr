#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XREquirectLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrEquirectLayerInit` dictionary."]
    pub type XrEquirectLayerInit;
    #[doc = "Get the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, getter = "clearOnAccess")]
    pub fn get_clear_on_access(this: &XrEquirectLayerInit) -> Option<bool>;
    #[doc = "Change the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, setter = "clearOnAccess")]
    pub fn set_clear_on_access(this: &XrEquirectLayerInit, val: bool);
    #[doc = "Get the `isStatic` field of this object."]
    #[wasm_bindgen(method, getter = "isStatic")]
    pub fn get_is_static(this: &XrEquirectLayerInit) -> Option<bool>;
    #[doc = "Change the `isStatic` field of this object."]
    #[wasm_bindgen(method, setter = "isStatic")]
    pub fn set_is_static(this: &XrEquirectLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrEquirectLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrEquirectLayerInit, val: XrLayerLayout);
    #[doc = "Get the `mipLevels` field of this object."]
    #[wasm_bindgen(method, getter = "mipLevels")]
    pub fn get_mip_levels(this: &XrEquirectLayerInit) -> Option<u32>;
    #[doc = "Change the `mipLevels` field of this object."]
    #[wasm_bindgen(method, setter = "mipLevels")]
    pub fn set_mip_levels(this: &XrEquirectLayerInit, val: u32);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrEquirectLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrEquirectLayerInit, val: &XrSpace);
    #[doc = "Get the `textureType` field of this object."]
    #[wasm_bindgen(method, getter = "textureType")]
    pub fn get_texture_type(this: &XrEquirectLayerInit) -> Option<XrTextureType>;
    #[doc = "Change the `textureType` field of this object."]
    #[wasm_bindgen(method, setter = "textureType")]
    pub fn set_texture_type(this: &XrEquirectLayerInit, val: XrTextureType);
    #[doc = "Get the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelHeight")]
    pub fn get_view_pixel_height(this: &XrEquirectLayerInit) -> u32;
    #[doc = "Change the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelHeight")]
    pub fn set_view_pixel_height(this: &XrEquirectLayerInit, val: u32);
    #[doc = "Get the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelWidth")]
    pub fn get_view_pixel_width(this: &XrEquirectLayerInit) -> u32;
    #[doc = "Change the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelWidth")]
    pub fn set_view_pixel_width(this: &XrEquirectLayerInit, val: u32);
    #[doc = "Get the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "centralHorizontalAngle")]
    pub fn get_central_horizontal_angle(this: &XrEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "centralHorizontalAngle")]
    pub fn set_central_horizontal_angle(this: &XrEquirectLayerInit, val: f32);
    #[doc = "Get the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "lowerVerticalAngle")]
    pub fn get_lower_vertical_angle(this: &XrEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "lowerVerticalAngle")]
    pub fn set_lower_vertical_angle(this: &XrEquirectLayerInit, val: f32);
    #[doc = "Get the `radius` field of this object."]
    #[wasm_bindgen(method, getter = "radius")]
    pub fn get_radius(this: &XrEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `radius` field of this object."]
    #[wasm_bindgen(method, setter = "radius")]
    pub fn set_radius(this: &XrEquirectLayerInit, val: f32);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrEquirectLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrEquirectLayerInit, val: Option<&XrRigidTransform>);
    #[doc = "Get the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "upperVerticalAngle")]
    pub fn get_upper_vertical_angle(this: &XrEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "upperVerticalAngle")]
    pub fn set_upper_vertical_angle(this: &XrEquirectLayerInit, val: f32);
}
impl XrEquirectLayerInit {
    #[doc = "Construct a new `XrEquirectLayerInit`."]
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
    #[deprecated = "Use `set_central_horizontal_angle()` instead."]
    pub fn central_horizontal_angle(&mut self, val: f32) -> &mut Self {
        self.set_central_horizontal_angle(val);
        self
    }
    #[deprecated = "Use `set_lower_vertical_angle()` instead."]
    pub fn lower_vertical_angle(&mut self, val: f32) -> &mut Self {
        self.set_lower_vertical_angle(val);
        self
    }
    #[deprecated = "Use `set_radius()` instead."]
    pub fn radius(&mut self, val: f32) -> &mut Self {
        self.set_radius(val);
        self
    }
    #[deprecated = "Use `set_transform()` instead."]
    pub fn transform(&mut self, val: Option<&XrRigidTransform>) -> &mut Self {
        self.set_transform(val);
        self
    }
    #[deprecated = "Use `set_upper_vertical_angle()` instead."]
    pub fn upper_vertical_angle(&mut self, val: f32) -> &mut Self {
        self.set_upper_vertical_angle(val);
        self
    }
}
