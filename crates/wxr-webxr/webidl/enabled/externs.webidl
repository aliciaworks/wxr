// The things WebXR's IDL names and other specifications own.
//
// This file is ours and not a snapshot: WebXR refers to the DOM it inherits from and to the WebGPU and
// WebGL objects it hands over, and neither is WebXR's to define. Declaring them here is what lets the
// generator resolve every name in the snapshot without also generating a second DOM or a second WebGPU.
//
// Two kinds live here. The DOM interfaces WebXR inherits from are empty: the generated types need a base,
// and nothing in this workspace uses the base itself. Everything WebGPU and WebGL is `any`: those objects
// are made and owned by wgpu and by the browser, and a generated Rust type for one would be a second name
// for the same object - which is the thing this file exists to avoid.

interface EventTarget {};
interface Event : EventTarget {};
interface PermissionStatus {};
interface mixin GlobalEventHandlers {};
interface mixin WebGLRenderingContextBase {};
interface Navigator {};
interface Gamepad {};

enum PermissionState { "granted", "denied", "prompt" };

// WebGPU: named, never mirrored.
typedef any GPUDevice;
typedef any GPUTexture;
typedef any GPUTextureViewDescriptor;
typedef any GPURequestAdapterOptions;
typedef DOMString GPUTextureFormat;
typedef unsigned long GPUTextureUsageFlags;
typedef unsigned long GPUTextureUsage;

// WebGL: the Layers module's WebGL half, which this workspace does not use but does generate.
typedef any WebGLTexture;
typedef any WebGLFramebuffer;
typedef any WebGLRenderingContext;
typedef any WebGL2RenderingContext;
typedef any XRWebGLRenderingContext;
typedef any EventHandler;
typedef any DOMPointReadOnly;

// The dictionaries WebXR's own extend. Two fields each, and only the ones the specification's inheritance
// algorithm needs in order to walk the chain.
dictionary EventInit {
  boolean bubbles = false;
  boolean cancelable = false;
  boolean composed = false;
};

dictionary PermissionDescriptor {
  required DOMString name;
};

// The DOM nodes WebXR's DOM overlays name, which this workspace generates but does not use.
typedef any Element;
typedef any Document;
typedef any HTMLElement;
