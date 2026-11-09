#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRGPUQuadLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuQuadLayerInit` dictionary."]
    pub type XrgpuQuadLayerInit;
    #[doc = "Get the `colorFormat` field of this object."]
    #[wasm_bindgen(method, getter = "colorFormat")]
    pub fn get_color_format(this: &XrgpuQuadLayerInit) -> ::alloc::string::String;
    #[doc = "Change the `colorFormat` field of this object."]
    #[wasm_bindgen(method, setter = "colorFormat")]
    pub fn set_color_format(this: &XrgpuQuadLayerInit, val: &str);
    #[doc = "Get the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, getter = "depthStencilFormat")]
    pub fn get_depth_stencil_format(this: &XrgpuQuadLayerInit) -> Option<::alloc::string::String>;
    #[doc = "Change the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, setter = "depthStencilFormat")]
    pub fn set_depth_stencil_format(this: &XrgpuQuadLayerInit, val: Option<&str>);
    #[doc = "Get the `isStatic` field of this object."]
    #[wasm_bindgen(method, getter = "isStatic")]
    pub fn get_is_static(this: &XrgpuQuadLayerInit) -> Option<bool>;
    #[doc = "Change the `isStatic` field of this object."]
    #[wasm_bindgen(method, setter = "isStatic")]
    pub fn set_is_static(this: &XrgpuQuadLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrgpuQuadLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrgpuQuadLayerInit, val: XrLayerLayout);
    #[doc = "Get the `mipLevels` field of this object."]
    #[wasm_bindgen(method, getter = "mipLevels")]
    pub fn get_mip_levels(this: &XrgpuQuadLayerInit) -> Option<u32>;
    #[doc = "Change the `mipLevels` field of this object."]
    #[wasm_bindgen(method, setter = "mipLevels")]
    pub fn set_mip_levels(this: &XrgpuQuadLayerInit, val: u32);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrgpuQuadLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrgpuQuadLayerInit, val: &XrSpace);
    #[doc = "Get the `textureUsage` field of this object."]
    #[wasm_bindgen(method, getter = "textureUsage")]
    pub fn get_texture_usage(this: &XrgpuQuadLayerInit) -> Option<u32>;
    #[doc = "Change the `textureUsage` field of this object."]
    #[wasm_bindgen(method, setter = "textureUsage")]
    pub fn set_texture_usage(this: &XrgpuQuadLayerInit, val: u32);
    #[doc = "Get the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelHeight")]
    pub fn get_view_pixel_height(this: &XrgpuQuadLayerInit) -> u32;
    #[doc = "Change the `viewPixelHeight` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelHeight")]
    pub fn set_view_pixel_height(this: &XrgpuQuadLayerInit, val: u32);
    #[doc = "Get the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, getter = "viewPixelWidth")]
    pub fn get_view_pixel_width(this: &XrgpuQuadLayerInit) -> u32;
    #[doc = "Change the `viewPixelWidth` field of this object."]
    #[wasm_bindgen(method, setter = "viewPixelWidth")]
    pub fn set_view_pixel_width(this: &XrgpuQuadLayerInit, val: u32);
    #[doc = "Get the `height` field of this object."]
    #[wasm_bindgen(method, getter = "height")]
    pub fn get_height(this: &XrgpuQuadLayerInit) -> Option<f32>;
    #[doc = "Change the `height` field of this object."]
    #[wasm_bindgen(method, setter = "height")]
    pub fn set_height(this: &XrgpuQuadLayerInit, val: f32);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrgpuQuadLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrgpuQuadLayerInit, val: Option<&XrRigidTransform>);
    #[doc = "Get the `width` field of this object."]
    #[wasm_bindgen(method, getter = "width")]
    pub fn get_width(this: &XrgpuQuadLayerInit) -> Option<f32>;
    #[doc = "Change the `width` field of this object."]
    #[wasm_bindgen(method, setter = "width")]
    pub fn set_width(this: &XrgpuQuadLayerInit, val: f32);
}
impl XrgpuQuadLayerInit {
    #[doc = "Construct a new `XrgpuQuadLayerInit`."]
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
    #[deprecated = "Use `set_height()` instead."]
    pub fn height(&mut self, val: f32) -> &mut Self {
        self.set_height(val);
        self
    }
    #[deprecated = "Use `set_transform()` instead."]
    pub fn transform(&mut self, val: Option<&XrRigidTransform>) -> &mut Self {
        self.set_transform(val);
        self
    }
    #[deprecated = "Use `set_width()` instead."]
    pub fn width(&mut self, val: f32) -> &mut Self {
        self.set_width(val);
        self
    }
}
