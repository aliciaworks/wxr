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

// The bitmap the image-tracking draft is given to look for: HTML's, made by `createImageBitmap`, and a
// `typedef any` for the reason above.
typedef any ImageBitmap;

// The Web Audio buffer the haptics draft plays: `web-sys`'s, like the gamepad, and a `typedef any` for the
// same reason - this crate hands it to the browser rather than looking inside it.
typedef any AudioBuffer;

// The DOM's gamepad is `web-sys`'s - it is a stable interface with a stable shape, and mirroring it here
// would be a second Rust type for one object. What WebXR names is the *type* of the attribute.
typedef any Gamepad;

// A `typedef` and not an `enum`, which needs explaining: a generated string enum is a JavaScript object that
// `wasm-bindgen` registers under its JavaScript name, and two of them with one name is a duplicate it refuses
// when the bindings are generated. `PermissionState` belongs to the Permissions specification and `web-sys`
// declares it too, so a graph that enables that feature - and this workspace's does not have to, someone else
// in it does - gets two. It is a string at the boundary either way, and nothing here reads it.
typedef DOMString PermissionState;

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

// The last of what WebXR names and does not own. Each of these was a *silent* omission until `Tools/check_webxr_sys.py`
// said so: a declaration the generator cannot resolve is a declaration it leaves out, and an attribute whose type
// is unknown is an attribute it leaves out with it - `DOMHighResTimeStamp` was how `XRFrame.predictedDisplayTime`
// went missing, which is the frame's own clock.
typedef double DOMHighResTimeStamp;
typedef unsigned long GLenum;
typedef any HTMLVideoElement;
// A dictionary and not a typedef: `webxr.webidl` declares a `partial dictionary` of this name, and a
// partial needs a base to add to.
dictionary WebGLContextAttributes {};

// The base of a `partial dictionary` in the WebGPU binding: declared so that the partial has something to add to,
// even though nothing here reads the adapter request.
dictionary GPURequestAdapterOptions {};
