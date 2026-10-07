#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRLightEstimate",
        typescript_type = "XRLightEstimate"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrLightEstimate` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightEstimate)"]
    pub type XrLightEstimate;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRLightEstimate",
        js_name = "sphericalHarmonicsCoefficients"
    )]
    #[doc = "Getter for the `sphericalHarmonicsCoefficients` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightEstimate/sphericalHarmonicsCoefficients)"]
    pub fn spherical_harmonics_coefficients(this: &XrLightEstimate) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRLightEstimate",
        js_name = "primaryLightDirection"
    )]
    #[doc = "Getter for the `primaryLightDirection` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightEstimate/primaryLightDirection)"]
    pub fn primary_light_direction(this: &XrLightEstimate) -> DomPointReadOnly;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRLightEstimate",
        js_name = "primaryLightIntensity"
    )]
    #[doc = "Getter for the `primaryLightIntensity` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightEstimate/primaryLightIntensity)"]
    pub fn primary_light_intensity(this: &XrLightEstimate) -> DomPointReadOnly;
}
