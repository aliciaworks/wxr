#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRFrame",
        typescript_type = "XRFrame"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrFrame` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame)"]
    pub type XrFrame;
    #[wasm_bindgen(method, getter, js_class = "XRFrame", js_name = "trackedAnchors")]
    #[doc = "Getter for the `trackedAnchors` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/trackedAnchors)"]
    pub fn tracked_anchors(this: &XrFrame) -> XrAnchorSet;
    #[wasm_bindgen(method, getter, js_class = "XRFrame", js_name = "detectedPlanes")]
    #[doc = "Getter for the `detectedPlanes` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/detectedPlanes)"]
    pub fn detected_planes(this: &XrFrame) -> XrPlaneSet;
    #[wasm_bindgen(method, getter, js_class = "XRFrame", js_name = "session")]
    #[doc = "Getter for the `session` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/session)"]
    pub fn session(this: &XrFrame) -> XrSession;
    #[wasm_bindgen(method, getter, js_class = "XRFrame", js_name = "predictedDisplayTime")]
    #[doc = "Getter for the `predictedDisplayTime` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/predictedDisplayTime)"]
    pub fn predicted_display_time(this: &XrFrame) -> f64;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "createAnchor")]
    #[doc = "The `createAnchor()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/createAnchor)"]
    pub fn create_anchor(
        this: &XrFrame,
        pose: &XrRigidTransform,
        space: &XrSpace,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(catch, method, js_class = "XRFrame", js_name = "fillJointRadii")]
    #[doc = "The `fillJointRadii()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/fillJointRadii)"]
    pub fn fill_joint_radii_with_f32_slice(
        this: &XrFrame,
        joint_spaces: &::wasm_bindgen::JsValue,
        radii: &mut [f32],
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "XRFrame", js_name = "fillJointRadii")]
    #[doc = "The `fillJointRadii()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/fillJointRadii)"]
    pub fn fill_joint_radii_with_f32_array(
        this: &XrFrame,
        joint_spaces: &::wasm_bindgen::JsValue,
        radii: &::js_sys::Float32Array,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "XRFrame", js_name = "fillPoses")]
    #[doc = "The `fillPoses()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/fillPoses)"]
    pub fn fill_poses_with_f32_slice(
        this: &XrFrame,
        spaces: &::wasm_bindgen::JsValue,
        base_space: &XrSpace,
        transforms: &mut [f32],
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "XRFrame", js_name = "fillPoses")]
    #[doc = "The `fillPoses()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/fillPoses)"]
    pub fn fill_poses_with_f32_array(
        this: &XrFrame,
        spaces: &::wasm_bindgen::JsValue,
        base_space: &XrSpace,
        transforms: &::js_sys::Float32Array,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getDepthInformation")]
    #[doc = "The `getDepthInformation()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getDepthInformation)"]
    pub fn get_depth_information(this: &XrFrame, view: &XrView) -> Option<XrcpuDepthInformation>;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getHitTestResults")]
    #[doc = "The `getHitTestResults()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getHitTestResults)"]
    pub fn get_hit_test_results(
        this: &XrFrame,
        hit_test_source: &XrHitTestSource,
    ) -> ::js_sys::Array;
    #[wasm_bindgen(
        method,
        js_class = "XRFrame",
        js_name = "getHitTestResultsForTransientInput"
    )]
    #[doc = "The `getHitTestResultsForTransientInput()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getHitTestResultsForTransientInput)"]
    pub fn get_hit_test_results_for_transient_input(
        this: &XrFrame,
        hit_test_source: &XrTransientInputHitTestSource,
    ) -> ::js_sys::Array;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getJointPose")]
    #[doc = "The `getJointPose()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getJointPose)"]
    pub fn get_joint_pose(
        this: &XrFrame,
        joint: &XrJointSpace,
        base_space: &XrSpace,
    ) -> Option<XrJointPose>;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getLightEstimate")]
    #[doc = "The `getLightEstimate()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getLightEstimate)"]
    pub fn get_light_estimate(
        this: &XrFrame,
        light_probe: &XrLightProbe,
    ) -> Option<XrLightEstimate>;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getPose")]
    #[doc = "The `getPose()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getPose)"]
    pub fn get_pose(this: &XrFrame, space: &XrSpace, base_space: &XrSpace) -> Option<XrPose>;
    #[wasm_bindgen(method, js_class = "XRFrame", js_name = "getViewerPose")]
    #[doc = "The `getViewerPose()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRFrame/getViewerPose)"]
    pub fn get_viewer_pose(
        this: &XrFrame,
        reference_space: &XrReferenceSpace,
    ) -> Option<XrViewerPose>;
}
