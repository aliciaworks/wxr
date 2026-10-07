// The things WebXR's IDL names and other specifications own.
//
// This file is ours and not a snapshot: WebXR refers to the DOM it inherits from and to the WebGPU and
// WebGL objects it hands over, and neither is WebXR's to define. Declaring them here is what lets the
// generator resolve every name in the snapshot without also generating a second DOM or a second WebGPU.
//
// Three kinds live here. The DOM interfaces WebXR inherits from are the smallest declaration that gives the
// generated types a base and the members this backend actually calls - `addEventListener` is how the input
// events arrive, so an empty `EventTarget` would be a generated session nothing could subscribe to.
// Everything WebGPU and WebGL is `any`: those objects are made and owned by wgpu and by the browser, and a
// generated Rust type for one would be a second name for the same object, which is the thing this file
// exists to avoid.

callback EventHandlerNonNull = any (Event event);
typedef EventHandlerNonNull? EventHandler;

callback interface EventListener {
  undefined handleEvent(Event event);
};

interface EventTarget {
  undefined addEventListener(DOMString type, EventListener? callback);
  undefined removeEventListener(DOMString type, EventListener? callback);
  boolean dispatchEvent(Event event);
};

interface Event : EventTarget {};

interface PermissionStatus {};
interface mixin GlobalEventHandlers {};
interface mixin WebGLRenderingContextBase {};
interface Navigator {};

// The DOM's gamepad is `web-sys`'s - it is a stable interface with a stable shape, and mirroring it here
// would be a second Rust type for one object. What WebXR names is the *type* of the attribute.
typedef any Gamepad;

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

// The geometry WebXR's poses are made of. Real declarations rather than `any`, because a pose is the one thing
// this backend reads out of a rigid transform: leaving `DOMPointReadOnly` untyped is what silently cost
// `XRRigidTransform` its constructor arguments, which is the failure mode a generated binding is supposed to
// make impossible.
dictionary DOMPointInit {
  unrestricted double x = 0;
  unrestricted double y = 0;
  unrestricted double z = 0;
  unrestricted double w = 1;
};

interface DOMPointReadOnly {
  readonly attribute unrestricted double x;
  readonly attribute unrestricted double y;
  readonly attribute unrestricted double z;
  readonly attribute unrestricted double w;
};
