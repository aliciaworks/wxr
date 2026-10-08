//! The WebXR API, as Rust.
//!
//! Generated from the IDL in `crates/wxr-webxr/webidl/` by `Tools/refresh_webxr_idl.py` - do not edit by
//! hand. Everything below is one `mod` and one `pub use` per interface, dictionary and enum the WebXR
//! specifications declare, under the names the generator derives from them (`XRGPUBinding` becomes
//! `XrgpuBinding`, which is why every generated `js_name` is a checkable fact rather than a guess).
//!
//! Regenerating is `python3 Tools/refresh_webxr_idl.py`; it says what it needs.

#[allow(non_snake_case)]
mod gen_DomPointInit;
#[allow(unused_imports)]
pub use gen_DomPointInit::*;

#[allow(non_snake_case)]
mod gen_DomPointReadOnly;
#[allow(unused_imports)]
pub use gen_DomPointReadOnly::*;

#[allow(non_snake_case)]
mod gen_Event;
#[allow(unused_imports)]
pub use gen_Event::*;

#[allow(non_snake_case)]
mod gen_EventInit;
#[allow(unused_imports)]
pub use gen_EventInit::*;

#[allow(non_snake_case)]
mod gen_EventListener;
#[allow(unused_imports)]
pub use gen_EventListener::*;

#[allow(non_snake_case)]
mod gen_EventTarget;
#[allow(unused_imports)]
pub use gen_EventTarget::*;

#[allow(non_snake_case)]
mod gen_GamepadHapticActuator;
#[allow(unused_imports)]
pub use gen_GamepadHapticActuator::*;

#[allow(non_snake_case)]
mod gen_GpuRequestAdapterOptions;
#[allow(unused_imports)]
pub use gen_GpuRequestAdapterOptions::*;

#[allow(non_snake_case)]
mod gen_Navigator;
#[allow(unused_imports)]
pub use gen_Navigator::*;

#[allow(non_snake_case)]
mod gen_PermissionDescriptor;
#[allow(unused_imports)]
pub use gen_PermissionDescriptor::*;

#[allow(non_snake_case)]
mod gen_PermissionStatus;
#[allow(unused_imports)]
pub use gen_PermissionStatus::*;

#[allow(non_snake_case)]
mod gen_WebGlContextAttributes;
#[allow(unused_imports)]
pub use gen_WebGlContextAttributes::*;

#[allow(non_snake_case)]
mod gen_XrAnchor;
#[allow(unused_imports)]
pub use gen_XrAnchor::*;

#[allow(non_snake_case)]
mod gen_XrAnchorSet;
#[allow(unused_imports)]
pub use gen_XrAnchorSet::*;

#[allow(non_snake_case)]
mod gen_XrBoundedReferenceSpace;
#[allow(unused_imports)]
pub use gen_XrBoundedReferenceSpace::*;

#[allow(non_snake_case)]
mod gen_XrCamera;
#[allow(unused_imports)]
pub use gen_XrCamera::*;

#[allow(non_snake_case)]
mod gen_XrCompositionLayer;
#[allow(unused_imports)]
pub use gen_XrCompositionLayer::*;

#[allow(non_snake_case)]
mod gen_XrCubeLayer;
#[allow(unused_imports)]
pub use gen_XrCubeLayer::*;

#[allow(non_snake_case)]
mod gen_XrCubeLayerInit;
#[allow(unused_imports)]
pub use gen_XrCubeLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrCylinderLayer;
#[allow(unused_imports)]
pub use gen_XrCylinderLayer::*;

#[allow(non_snake_case)]
mod gen_XrCylinderLayerInit;
#[allow(unused_imports)]
pub use gen_XrCylinderLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrDepthDataFormat;
#[allow(unused_imports)]
pub use gen_XrDepthDataFormat::*;

#[allow(non_snake_case)]
mod gen_XrDepthInformation;
#[allow(unused_imports)]
pub use gen_XrDepthInformation::*;

#[allow(non_snake_case)]
mod gen_XrDepthStateInit;
#[allow(unused_imports)]
pub use gen_XrDepthStateInit::*;

#[allow(non_snake_case)]
mod gen_XrDepthType;
#[allow(unused_imports)]
pub use gen_XrDepthType::*;

#[allow(non_snake_case)]
mod gen_XrDepthUsage;
#[allow(unused_imports)]
pub use gen_XrDepthUsage::*;

#[allow(non_snake_case)]
mod gen_XrEnvironmentBlendMode;
#[allow(unused_imports)]
pub use gen_XrEnvironmentBlendMode::*;

#[allow(non_snake_case)]
mod gen_XrEquirectLayer;
#[allow(unused_imports)]
pub use gen_XrEquirectLayer::*;

#[allow(non_snake_case)]
mod gen_XrEquirectLayerInit;
#[allow(unused_imports)]
pub use gen_XrEquirectLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrExpression;
#[allow(unused_imports)]
pub use gen_XrExpression::*;

#[allow(non_snake_case)]
mod gen_XrExpressions;
#[allow(unused_imports)]
pub use gen_XrExpressions::*;

#[allow(non_snake_case)]
mod gen_XrEye;
#[allow(unused_imports)]
pub use gen_XrEye::*;

#[allow(non_snake_case)]
mod gen_XrFrame;
#[allow(unused_imports)]
pub use gen_XrFrame::*;

#[allow(non_snake_case)]
mod gen_XrHand;
#[allow(unused_imports)]
pub use gen_XrHand::*;

#[allow(non_snake_case)]
mod gen_XrHandJoint;
#[allow(unused_imports)]
pub use gen_XrHandJoint::*;

#[allow(non_snake_case)]
mod gen_XrHandedness;
#[allow(unused_imports)]
pub use gen_XrHandedness::*;

#[allow(non_snake_case)]
mod gen_XrHitTestOptionsInit;
#[allow(unused_imports)]
pub use gen_XrHitTestOptionsInit::*;

#[allow(non_snake_case)]
mod gen_XrHitTestResult;
#[allow(unused_imports)]
pub use gen_XrHitTestResult::*;

#[allow(non_snake_case)]
mod gen_XrHitTestSource;
#[allow(unused_imports)]
pub use gen_XrHitTestSource::*;

#[allow(non_snake_case)]
mod gen_XrHitTestTrackableType;
#[allow(unused_imports)]
pub use gen_XrHitTestTrackableType::*;

#[allow(non_snake_case)]
mod gen_XrInputSource;
#[allow(unused_imports)]
pub use gen_XrInputSource::*;

#[allow(non_snake_case)]
mod gen_XrInputSourceArray;
#[allow(unused_imports)]
pub use gen_XrInputSourceArray::*;

#[allow(non_snake_case)]
mod gen_XrInputSourceEvent;
#[allow(unused_imports)]
pub use gen_XrInputSourceEvent::*;

#[allow(non_snake_case)]
mod gen_XrInputSourceEventInit;
#[allow(unused_imports)]
pub use gen_XrInputSourceEventInit::*;

#[allow(non_snake_case)]
mod gen_XrInputSourcesChangeEvent;
#[allow(unused_imports)]
pub use gen_XrInputSourcesChangeEvent::*;

#[allow(non_snake_case)]
mod gen_XrInputSourcesChangeEventInit;
#[allow(unused_imports)]
pub use gen_XrInputSourcesChangeEventInit::*;

#[allow(non_snake_case)]
mod gen_XrInteractionMode;
#[allow(unused_imports)]
pub use gen_XrInteractionMode::*;

#[allow(non_snake_case)]
mod gen_XrJointPose;
#[allow(unused_imports)]
pub use gen_XrJointPose::*;

#[allow(non_snake_case)]
mod gen_XrJointSpace;
#[allow(unused_imports)]
pub use gen_XrJointSpace::*;

#[allow(non_snake_case)]
mod gen_XrLayer;
#[allow(unused_imports)]
pub use gen_XrLayer::*;

#[allow(non_snake_case)]
mod gen_XrLayerEvent;
#[allow(unused_imports)]
pub use gen_XrLayerEvent::*;

#[allow(non_snake_case)]
mod gen_XrLayerEventInit;
#[allow(unused_imports)]
pub use gen_XrLayerEventInit::*;

#[allow(non_snake_case)]
mod gen_XrLayerInit;
#[allow(unused_imports)]
pub use gen_XrLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrLayerLayout;
#[allow(unused_imports)]
pub use gen_XrLayerLayout::*;

#[allow(non_snake_case)]
mod gen_XrLayerQuality;
#[allow(unused_imports)]
pub use gen_XrLayerQuality::*;

#[allow(non_snake_case)]
mod gen_XrLightEstimate;
#[allow(unused_imports)]
pub use gen_XrLightEstimate::*;

#[allow(non_snake_case)]
mod gen_XrLightProbe;
#[allow(unused_imports)]
pub use gen_XrLightProbe::*;

#[allow(non_snake_case)]
mod gen_XrLightProbeInit;
#[allow(unused_imports)]
pub use gen_XrLightProbeInit::*;

#[allow(non_snake_case)]
mod gen_XrMediaBinding;
#[allow(unused_imports)]
pub use gen_XrMediaBinding::*;

#[allow(non_snake_case)]
mod gen_XrMediaCylinderLayerInit;
#[allow(unused_imports)]
pub use gen_XrMediaCylinderLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrMediaEquirectLayerInit;
#[allow(unused_imports)]
pub use gen_XrMediaEquirectLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrMediaLayerInit;
#[allow(unused_imports)]
pub use gen_XrMediaLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrMediaQuadLayerInit;
#[allow(unused_imports)]
pub use gen_XrMediaQuadLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrMesh;
#[allow(unused_imports)]
pub use gen_XrMesh::*;

#[allow(non_snake_case)]
mod gen_XrMeshSet;
#[allow(unused_imports)]
pub use gen_XrMeshSet::*;

#[allow(non_snake_case)]
mod gen_XrPermissionDescriptor;
#[allow(unused_imports)]
pub use gen_XrPermissionDescriptor::*;

#[allow(non_snake_case)]
mod gen_XrPermissionStatus;
#[allow(unused_imports)]
pub use gen_XrPermissionStatus::*;

#[allow(non_snake_case)]
mod gen_XrPlane;
#[allow(unused_imports)]
pub use gen_XrPlane::*;

#[allow(non_snake_case)]
mod gen_XrPlaneOrientation;
#[allow(unused_imports)]
pub use gen_XrPlaneOrientation::*;

#[allow(non_snake_case)]
mod gen_XrPlaneSet;
#[allow(unused_imports)]
pub use gen_XrPlaneSet::*;

#[allow(non_snake_case)]
mod gen_XrPose;
#[allow(unused_imports)]
pub use gen_XrPose::*;

#[allow(non_snake_case)]
mod gen_XrProjectionLayer;
#[allow(unused_imports)]
pub use gen_XrProjectionLayer::*;

#[allow(non_snake_case)]
mod gen_XrProjectionLayerInit;
#[allow(unused_imports)]
pub use gen_XrProjectionLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrQuadLayer;
#[allow(unused_imports)]
pub use gen_XrQuadLayer::*;

#[allow(non_snake_case)]
mod gen_XrQuadLayerInit;
#[allow(unused_imports)]
pub use gen_XrQuadLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrRay;
#[allow(unused_imports)]
pub use gen_XrRay::*;

#[allow(non_snake_case)]
mod gen_XrRayDirectionInit;
#[allow(unused_imports)]
pub use gen_XrRayDirectionInit::*;

#[allow(non_snake_case)]
mod gen_XrReferenceSpace;
#[allow(unused_imports)]
pub use gen_XrReferenceSpace::*;

#[allow(non_snake_case)]
mod gen_XrReferenceSpaceEvent;
#[allow(unused_imports)]
pub use gen_XrReferenceSpaceEvent::*;

#[allow(non_snake_case)]
mod gen_XrReferenceSpaceEventInit;
#[allow(unused_imports)]
pub use gen_XrReferenceSpaceEventInit::*;

#[allow(non_snake_case)]
mod gen_XrReferenceSpaceType;
#[allow(unused_imports)]
pub use gen_XrReferenceSpaceType::*;

#[allow(non_snake_case)]
mod gen_XrReflectionFormat;
#[allow(unused_imports)]
pub use gen_XrReflectionFormat::*;

#[allow(non_snake_case)]
mod gen_XrRenderState;
#[allow(unused_imports)]
pub use gen_XrRenderState::*;

#[allow(non_snake_case)]
mod gen_XrRenderStateInit;
#[allow(unused_imports)]
pub use gen_XrRenderStateInit::*;

#[allow(non_snake_case)]
mod gen_XrRigidTransform;
#[allow(unused_imports)]
pub use gen_XrRigidTransform::*;

#[allow(non_snake_case)]
mod gen_XrSession;
#[allow(unused_imports)]
pub use gen_XrSession::*;

#[allow(non_snake_case)]
mod gen_XrSessionEvent;
#[allow(unused_imports)]
pub use gen_XrSessionEvent::*;

#[allow(non_snake_case)]
mod gen_XrSessionEventInit;
#[allow(unused_imports)]
pub use gen_XrSessionEventInit::*;

#[allow(non_snake_case)]
mod gen_XrSessionInit;
#[allow(unused_imports)]
pub use gen_XrSessionInit::*;

#[allow(non_snake_case)]
mod gen_XrSessionMode;
#[allow(unused_imports)]
pub use gen_XrSessionMode::*;

#[allow(non_snake_case)]
mod gen_XrSessionSupportedPermissionDescriptor;
#[allow(unused_imports)]
pub use gen_XrSessionSupportedPermissionDescriptor::*;

#[allow(non_snake_case)]
mod gen_XrSpace;
#[allow(unused_imports)]
pub use gen_XrSpace::*;

#[allow(non_snake_case)]
mod gen_XrSubImage;
#[allow(unused_imports)]
pub use gen_XrSubImage::*;

#[allow(non_snake_case)]
mod gen_XrSystem;
#[allow(unused_imports)]
pub use gen_XrSystem::*;

#[allow(non_snake_case)]
mod gen_XrTargetRayMode;
#[allow(unused_imports)]
pub use gen_XrTargetRayMode::*;

#[allow(non_snake_case)]
mod gen_XrTextureType;
#[allow(unused_imports)]
pub use gen_XrTextureType::*;

#[allow(non_snake_case)]
mod gen_XrTransientInputHitTestOptionsInit;
#[allow(unused_imports)]
pub use gen_XrTransientInputHitTestOptionsInit::*;

#[allow(non_snake_case)]
mod gen_XrTransientInputHitTestResult;
#[allow(unused_imports)]
pub use gen_XrTransientInputHitTestResult::*;

#[allow(non_snake_case)]
mod gen_XrTransientInputHitTestSource;
#[allow(unused_imports)]
pub use gen_XrTransientInputHitTestSource::*;

#[allow(non_snake_case)]
mod gen_XrView;
#[allow(unused_imports)]
pub use gen_XrView::*;

#[allow(non_snake_case)]
mod gen_XrViewerPose;
#[allow(unused_imports)]
pub use gen_XrViewerPose::*;

#[allow(non_snake_case)]
mod gen_XrViewport;
#[allow(unused_imports)]
pub use gen_XrViewport::*;

#[allow(non_snake_case)]
mod gen_XrVisibilityMaskChangeEvent;
#[allow(unused_imports)]
pub use gen_XrVisibilityMaskChangeEvent::*;

#[allow(non_snake_case)]
mod gen_XrVisibilityMaskChangeEventInit;
#[allow(unused_imports)]
pub use gen_XrVisibilityMaskChangeEventInit::*;

#[allow(non_snake_case)]
mod gen_XrVisibilityState;
#[allow(unused_imports)]
pub use gen_XrVisibilityState::*;

#[allow(non_snake_case)]
mod gen_XrWebGlBinding;
#[allow(unused_imports)]
pub use gen_XrWebGlBinding::*;

#[allow(non_snake_case)]
mod gen_XrWebGlDepthInformation;
#[allow(unused_imports)]
pub use gen_XrWebGlDepthInformation::*;

#[allow(non_snake_case)]
mod gen_XrWebGlLayer;
#[allow(unused_imports)]
pub use gen_XrWebGlLayer::*;

#[allow(non_snake_case)]
mod gen_XrWebGlLayerInit;
#[allow(unused_imports)]
pub use gen_XrWebGlLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrWebGlSubImage;
#[allow(unused_imports)]
pub use gen_XrWebGlSubImage::*;

#[allow(non_snake_case)]
mod gen_XrcpuDepthInformation;
#[allow(unused_imports)]
pub use gen_XrcpuDepthInformation::*;

#[allow(non_snake_case)]
mod gen_XrdomOverlayInit;
#[allow(unused_imports)]
pub use gen_XrdomOverlayInit::*;

#[allow(non_snake_case)]
mod gen_XrdomOverlayState;
#[allow(unused_imports)]
pub use gen_XrdomOverlayState::*;

#[allow(non_snake_case)]
mod gen_XrdomOverlayType;
#[allow(unused_imports)]
pub use gen_XrdomOverlayType::*;

#[allow(non_snake_case)]
mod gen_XrgpuBinding;
#[allow(unused_imports)]
pub use gen_XrgpuBinding::*;

#[allow(non_snake_case)]
mod gen_XrgpuCubeLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuCubeLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuCylinderLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuCylinderLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuDepthInformation;
#[allow(unused_imports)]
pub use gen_XrgpuDepthInformation::*;

#[allow(non_snake_case)]
mod gen_XrgpuEquirectLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuEquirectLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuProjectionLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuProjectionLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuQuadLayerInit;
#[allow(unused_imports)]
pub use gen_XrgpuQuadLayerInit::*;

#[allow(non_snake_case)]
mod gen_XrgpuSubImage;
#[allow(unused_imports)]
pub use gen_XrgpuSubImage::*;
