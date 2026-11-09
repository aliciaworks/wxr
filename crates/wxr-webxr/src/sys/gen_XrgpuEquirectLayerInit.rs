#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRGPUEquirectLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuEquirectLayerInit` dictionary."]
    pub type XrgpuEquirectLayerInit;
    #[doc = "Get the `colorFormat` field of this object."]
    #[wasm_bindgen(method, getter = "colorFormat")]
    pub fn get_color_format(this: &XrgpuEquirectLayerInit) -> ::alloc::string::String;
    #[doc = "Change the `colorFormat` field of this object."]
    #[wasm_bindgen(method, setter = "colorFormat")]
    pub fn set_color_format(this: &XrgpuEquirectLayerInit, val: &str);
    #[doc = "Get the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, getter = "depthStencilFormat")]
    pub fn get_depth_stencil_format(
        this: &XrgpuEquirectLayerInit,
    ) -> Option<::alloc::string::String>;
    #[doc = "Change the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, setter = "depthStencilFormat")]
    pub fn set_depth_stencil_format(this: &XrgpuEquirectLayerInit, val: Option<&str>);
    #[doc = "Get the `isStatic` field of this object."]
    #[wasm_bindgen(method, getter = "isStatic")]
    pub fn get_is_static(this: &XrgpuEquirectLayerInit) -> Option<bool>;
    #[doc = "Change the `isStatic` field of this object."]
    #[wasm_bindgen(method, setter = "isStatic")]
    pub fn set_is_static(this: &XrgpuEquirectLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrgpuEquirectLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrgpuEquirectLayerInit, val: XrLayerLayout);
    #[doc = "Get the `mipLevels` field of this object."]
    #[wasm_bindgen(method, getter = "mipLevels")]
    pub fn get_mip_levels(this: &XrgpuEquirectLayerInit) -> Option<u32>;
    #[doc = "Change the `mipLevels` field of this object."]
    #[wasm_bindgen(method, setter = "mipLevels")]
    pub fn set_mip_levels(this: &XrgpuEquirectLayerInit, val: u32);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrgpuEquirectLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrgpuEquirectLayerInit, val: &XrSpace);
    #[doc = "Get the `textureUsage` field of this object."]
    #[wasm_bindgen(method, getter = "textureUsage")]
    pub fn get_texture_usage(this: &XrgpuEquirectLayerInit) -> Option<u32>;
    #[doc = "Change the `textureUsage` field of this object."]
    #[wasm_bindgen(method, setter = "textureUsage")]
    pub fn set_texture_usage(this: &XrgpuEquirectLayerInit, val: u32);
    #[doc = "Get the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelHeight")]
    pub fn get_view_pixel_height(this: &XrgpuEquirectLayerInit) -> u32;
    #[doc = "Change the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelHeight")]
    pub fn set_view_pixel_height(this: &XrgpuEquirectLayerInit, val: u32);
    #[doc = "Get the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelWidth")]
    pub fn get_view_pixel_width(this: &XrgpuEquirectLayerInit) -> u32;
    #[doc = "Change the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelWidth")]
    pub fn set_view_pixel_width(this: &XrgpuEquirectLayerInit, val: u32);
    #[doc = "Get the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "centralHorizontalAngle")]
    pub fn get_central_horizontal_angle(this: &XrgpuEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "centralHorizontalAngle")]
    pub fn set_central_horizontal_angle(this: &XrgpuEquirectLayerInit, val: f32);
    #[doc = "Get the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "lowerVerticalAngle")]
    pub fn get_lower_vertical_angle(this: &XrgpuEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "lowerVerticalAngle")]
    pub fn set_lower_vertical_angle(this: &XrgpuEquirectLayerInit, val: f32);
    #[doc = "Get the `radius` field of this object."]
    #[wasm_bindgen(method, getter = "radius")]
    pub fn get_radius(this: &XrgpuEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `radius` field of this object."]
    #[wasm_bindgen(method, setter = "radius")]
    pub fn set_radius(this: &XrgpuEquirectLayerInit, val: f32);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrgpuEquirectLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrgpuEquirectLayerInit, val: Option<&XrRigidTransform>);
    #[doc = "Get the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "upperVerticalAngle")]
    pub fn get_upper_vertical_angle(this: &XrgpuEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "upperVerticalAngle")]
    pub fn set_upper_vertical_angle(this: &XrgpuEquirectLayerInit, val: f32);
}
impl XrgpuEquirectLayerInit {
    #[doc = "Construct a new `XrgpuEquirectLayerInit`."]
    pub fn new(
        color_format: &str,
        space: &XrSpace,
        view_pixel_height: u32,
        view_pixel_width: u32,
    ) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_color_format(color_format);
        ret.set_space(space);
        ret.set_view_pixel_height(view_pixel_height);
        ret.set_view_pixel_width(view_pixel_width);
        ret
    }
    #[deprecated = "Use `set_color_format()` instead."]
    pub fn color_format(&mut self, val: &str) -> &mut Self {
        self.set_color_format(val);
        self
    }
    #[deprecated = "Use `set_depth_stencil_format()` instead."]
    pub fn depth_stencil_format(&mut self, val: Option<&str>) -> &mut Self {
        self.set_depth_stencil_format(val);
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
    #[deprecated = "Use `set_texture_usage()` instead."]
    pub fn texture_usage(&mut self, val: u32) -> &mut Self {
        self.set_texture_usage(val);
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
