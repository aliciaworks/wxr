// A WebXR that lives on the page, for browsers that have no headset to give one.
//
// `navigator.xr` exists on every Chromium, and on a desktop it answers `immersive-vr` with no - there is no
// device behind it. The Immersive Web Emulator is the usual answer, and it is an extension with a popup, a
// device catalogue and a race between two content scripts over who dispatches the device definition first,
// which is a thing to have archaeology for rather than a test to depend on. So this file instead: an XRSystem
// of our own, with two controllers, two hands and a head that moves, and no compositing at all.
//
// What it is good for is exactly what it has. It runs the session ladder, the reference spaces, the poses, the
// input sources, the hand joints, the haptics and the frame loop - every call `wxr-webxr` makes on a browser
// that has no images to give. Without `?gpu` it has no `XRGPUBinding` that means anything, so a session granted
// `webgpu` would be a session granted a feature it cannot honour: it is not in `enabledFeatures`, and a session
// like that is what `Presentation::Composited` with no pictures *is*. That is the honest answer, and the run
// that has nothing to draw is the one that proves it.
//
// **`?gpu` is the other answer.** The same page with the same XRSystem, plus an `XRGPUBinding` of our own that
// hands over textures the page's WebGPU device really made - so the image path has something to read: an
// `ImageMeta` with an extent and a format, one image, one viewport and one array layer per eye, and the room's
// depth for a session that asked for it. The binding replaces the browser's own, which it has to: Chromium
// ships one behind `--enable-features=WebXRWebGPUBinding` and its constructor validates the session against
// the browser's, so a stub session handed to it is a `TypeError` and a binding that never exists is a session
// with no images. A page that asked for the stub gets the stub, all the way down.
//
// It is loaded by `stub.html` and not by `index.html`, which is the whole of how it is opted into: a page that
// asked for the stub gets it, and a page that did not gets whatever the browser really has.

'use strict';

/**
 * Whether this page asked for the stub's WebGPU binding, which is all `?gpu` is.
 *
 * Read from the URL rather than from a global so that it is the driver's decision and not the stub's: two runs
 * of one page, and the difference between them is the address they were opened at.
 */
const GPU = new URLSearchParams(globalThis.location ? globalThis.location.search : '').has('gpu');

/** A rigid transform, as seven numbers: position first, then the quaternion. */
function invert(t) {
  const iq = [-t[3], -t[4], -t[5], t[6]];
  const p = rotate(iq, [t[0], t[1], t[2]]);
  return [-p[0], -p[1], -p[2], iq[0], iq[1], iq[2], iq[3]];
}

/** `a` after `b`: where something at `b` lands once `a` has moved. */
function compose(a, b) {
  const p = rotate([a[3], a[4], a[5], a[6]], [b[0], b[1], b[2]]);
  const q = multiply([a[3], a[4], a[5], a[6]], [b[3], b[4], b[5], b[6]]);
  return [a[0] + p[0], a[1] + p[1], a[2] + p[2], q[0], q[1], q[2], q[3]];
}

/** `what` measured from `to`, which is the only shape `getPose` has. */
function relative(what, to) {
  return compose(invert(to), what);
}

function multiply(a, b) {
  return [
    a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
    a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
    a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
    a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
  ];
}

function rotate(q, v) {
  const u = [q[0], q[1], q[2]];
  const s = q[3];
  const dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
  const uu = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
  const cross = [
    u[1] * v[2] - u[2] * v[1],
    u[2] * v[0] - u[0] * v[2],
    u[0] * v[1] - u[1] * v[0],
  ];
  return [
    2 * dot * u[0] + (s * s - uu) * v[0] + 2 * s * cross[0],
    2 * dot * u[1] + (s * s - uu) * v[1] + 2 * s * cross[1],
    2 * dot * u[2] + (s * s - uu) * v[2] + 2 * s * cross[2],
  ];
}

/** The column-major 4x4 a browser hands over with a pose. */
function matrixOf(t) {
  const [x, y, z, qx, qy, qz, qw] = t;
  const xx = qx * qx;
  const yy = qy * qy;
  const zz = qz * qz;
  const xy = qx * qy;
  const xz = qx * qz;
  const yz = qy * qz;
  const wx = qw * qx;
  const wy = qw * qy;
  const wz = qw * qz;
  return new Float32Array([
    1 - 2 * (yy + zz), 2 * (xy + wz), 2 * (xz - wy), 0,
    2 * (xy - wz), 1 - 2 * (xx + zz), 2 * (yz + wx), 0,
    2 * (xz + wy), 2 * (yz - wx), 1 - 2 * (xx + yy), 0,
    x, y, z, 1,
  ]);
}

/** A browser's `XRRigidTransform`, which is the same seven numbers read one at a time. */
function rigid(t) {
  const back = invert(t);
  return {
    position: { x: t[0], y: t[1], z: t[2], w: 1 },
    orientation: { x: t[3], y: t[4], z: t[5], w: t[6] },
    matrix: matrixOf(t),
    get inverse() {
      return rigid(back);
    },
  };
}

/** The projection matrix four openings make, which is the standard inverse `field_of_view` reads back. */
function projection(up, down, left, right, near, far) {
  const u = Math.tan(up);
  const d = Math.tan(down);
  const l = Math.tan(left);
  const r = Math.tan(right);
  const m = new Float32Array(16);
  m[0] = 2 / (r + l);
  m[5] = 2 / (u + d);
  m[8] = (r - l) / (r + l);
  m[9] = (u - d) / (u + d);
  m[10] = -(far + near) / (far - near);
  m[11] = -1;
  m[14] = -2 * far * near / (far - near);
  return m;
}

/** An eye opening in radians, as a headset's is: wider than it is tall, and more down than up. */
const FOV = [46 * Math.PI / 180, 42 * Math.PI / 180, 52 * Math.PI / 180, 52 * Math.PI / 180];
/** How far apart the eyes are, and how far in front of the head each one sits. */
const IPD = 0.032;
/** The eye height the floor space is asked for, which is the point of a floor space. */
const EYE_HEIGHT = 1.7;

/**
 * Where the head is, in the world, at time `t`.
 *
 * It moves on purpose. A frame loop that reads a pose once and caches it looks exactly like one that reads it
 * every frame until something moves, and eight frames of a head standing still prove nothing about which this
 * is - so the printed eye positions walk, and a reader can see them walk.
 */
function headAt(t) {
  const seconds = t / 1000;
  return [
    0.05 * Math.sin(seconds / 2),
    EYE_HEIGHT + 0.03 * Math.sin(seconds),
    0.02 * Math.cos(seconds / 3),
    0, 0, 0, 1,
  ];
}

/**
 * A space, and where it is in the world.
 *
 * The two kinds differ in the only way that matters: an input source's space has a pose of its own that a frame
 * reports, while a reference space is an *origin* that other poses are measured from. Both are asked the same
 * question by `getPose`, so both answer it the same way.
 */
class XRSpace {
  constructor(world) {
    this._world = world;
  }

  /** Where this space is in the world at the frame being built. `t` is the frame's own time. */
  at(_t) {
    return this._world;
  }
}

/** A reference space, which is an origin rather than a thing at a place. */
class XRReferenceSpace extends XRSpace {
  constructor(type, origin) {
    super(null);
    this._type = type;
    // Where the *origin* of this space is in the world, which is what the head and the hands are measured
    // against. `viewer` and `local` are the two that move with the head at the moment they were made.
    this._origin = origin;
    this.onreset = null;
  }

  at(t) {
    if (this._type === 'viewer') {
      return headAt(t);
    }
    return this._origin;
  }

  getOffsetReferenceSpace(transform) {
    const offset = [
      transform.position.x,
      transform.position.y,
      transform.position.z,
      transform.orientation.x,
      transform.orientation.y,
      transform.orientation.z,
      transform.orientation.w,
    ];
    const derived = new XRReferenceSpace(this._type, null);
    const base = this;
    derived.at = (t) => compose(base.at(t), offset);
    return derived;
  }
}

/** The dictionary `updateRenderState` is handed, and the only shape it has to have. */
class XRRenderStateInit {
  constructor() {
    this.depthNear = undefined;
    this.depthFar = undefined;
    this.layers = undefined;
    this.baseLayer = undefined;
  }
}

class XRView {
  constructor(eye, index, transform, near, far) {
    this.eye = eye;
    this.index = index;
    this.transform = rigid(transform);
    this.projectionMatrix = projection(FOV[0], FOV[1], FOV[2], FOV[3], near, far);
    this.recommendedViewportScale = undefined;
    this.isFirstPersonObserver = false;
  }
}

class XRViewerPose {
  constructor(transform, near, far) {
    this.transform = rigid(transform);
    const views = [];
    for (const [eye, index, x] of [['left', 0, -IPD], ['right', 1, IPD]]) {
      views.push(new XRView(eye, index, compose(transform, [x, 0, 0, 0, 0, 0, 1]), near, far));
    }
    this.views = views;
  }
}

class XRPose {
  constructor(transform, emulated = false) {
    this.transform = rigid(transform);
    this.emulatedPosition = emulated;
  }
}

class XRJointPose extends XRPose {
  constructor(transform, radius) {
    super(transform);
    this.radius = radius;
  }
}

/** The 25 joints a hand has, and where each one is from the wrist. */
const JOINTS = {
  wrist: [0, 0, 0],
  'thumb-metacarpal': [-0.02, 0.01, 0.03],
  'thumb-phalanx-proximal': [-0.03, 0.03, 0.05],
  'thumb-phalanx-distal': [-0.04, 0.05, 0.07],
  'thumb-tip': [-0.05, 0.06, 0.09],
  'index-finger-metacarpal': [-0.02, 0, -0.02],
  'index-finger-phalanx-proximal': [-0.02, 0.01, 0.03],
  'index-finger-phalanx-intermediate': [-0.02, 0.01, 0.06],
  'index-finger-phalanx-distal': [-0.02, 0.01, 0.08],
  'index-finger-tip': [-0.02, 0.01, 0.1],
  'middle-finger-metacarpal': [0, 0, -0.02],
  'middle-finger-phalanx-proximal': [0, 0.01, 0.03],
  'middle-finger-phalanx-intermediate': [0, 0.01, 0.065],
  'middle-finger-phalanx-distal': [0, 0.01, 0.085],
  'middle-finger-tip': [0, 0.01, 0.105],
  'ring-finger-metacarpal': [0.02, 0, -0.02],
  'ring-finger-phalanx-proximal': [0.02, 0.01, 0.03],
  'ring-finger-phalanx-intermediate': [0.02, 0.01, 0.06],
  'ring-finger-phalanx-distal': [0.02, 0.01, 0.08],
  'ring-finger-tip': [0.02, 0.01, 0.1],
  'pinky-finger-metacarpal': [0.035, 0, -0.02],
  'pinky-finger-phalanx-proximal': [0.035, 0.01, 0.025],
  'pinky-finger-phalanx-intermediate': [0.035, 0.01, 0.05],
  'pinky-finger-phalanx-distal': [0.035, 0.01, 0.065],
  'pinky-finger-tip': [0.035, 0.01, 0.08],
};

/**
 * A hand: the joints, each a space at the wrist's pose plus the joint's own offset from it.
 *
 * The spaces are also kept the other way round, by identity, because that is the only way `getJointPose` can
 * answer: a frame is handed a space and not a name, so a joint is found by being the same object the pose was
 * asked about - which is exactly how a browser does it, since an `XRSpace` is not a value.
 */
class XRHand {
  constructor(where) {
    this._joints = new Map();
    this._bySpace = new Map();
    for (const [name, offset] of Object.entries(JOINTS)) {
      const radius = name === 'wrist' ? 0.01 : name.endsWith('-tip') ? 0.008 : 0.012;
      const space = new XRSpace(where);
      const joint = { name, space, offset, radius, hand: this };
      this._joints.set(name, joint);
      this._bySpace.set(space, joint);
    }
    this._wrist = this._joints.get('wrist');
  }

  get size() {
    return this._joints.size;
  }

  get(name) {
    const joint = this._joints.get(name);
    if (!joint) {
      throw new TypeError(`there is no joint called ${name}`);
    }
    return joint.space;
  }

  keys() {
    return this._joints.keys();
  }

  values() {
    return [...this._joints.values()].map((joint) => joint.space)[Symbol.iterator]();
  }

  entries() {
    return [...this._joints.entries()].map(([name, joint]) => [name, joint.space])[Symbol.iterator]();
  }

  forEach(callback) {
    for (const [name, joint] of this._joints) {
      callback(joint.space, name, this);
    }
  }

  /** The joint a space belongs to, or `null` if no hand of this session has one. */
  jointOf(space) {
    return this._bySpace.get(space) || null;
  }
}

/** One joint's world pose at the frame being built: the wrist's, moved along by the offset from it. */
function jointPose(joint, t) {
  const where = joint.hand._wrist.space.at(t);
  const offset = rotate([where[3], where[4], where[5], where[6]], joint.offset);
  return [
    where[0] + offset[0],
    where[1] + offset[1],
    where[2] + offset[2],
    where[3], where[4], where[5], where[6],
  ];
}

/**
 * A gamepad, in the shape the platform's own `Gamepad` is.
 *
 * The prototype is borrowed rather than invented: `wxr-webxr` reads a gamepad through `web-sys`, which asks
 * `instanceof Gamepad` before it will read anything, and an object that fails that check is an object with no
 * buttons. `GamepadButton` is the same story one level down. Both globals are in every browser that has the
 * Gamepad API at all, and if one is missing the source simply has no buttons, which is the same answer as a
 * controller with no buttons.
 */
function asPlatform(object, name) {
  const platform = globalThis[name];
  if (platform && platform.prototype) {
    Object.setPrototypeOf(object, platform.prototype);
  }
  return object;
}

/** One controller, with the `xr-standard` mapping and a motor to buzz. */
function controller(handedness, x) {
  const actuator = asPlatform({
    pulse(intensity, durationMs) {
      // The promise a pulse returns is dropped by the caller too, and a stub that resolved it with nothing
      // would be a stub that could not tell a buzz that was sent from one that was not.
      return Promise.resolve({ intensity, durationMs });
    },
  }, 'GamepadHapticActuator');
  const gamepad = asPlatform({
    id: `wxr-stub ${handedness}`,
    connected: true,
    timestamp: 0,
    mapping: 'xr-standard',
    axes: [0, 0, 0, 0],
    buttons: Array.from({ length: 4 }, () =>
      asPlatform({ pressed: false, touched: false, value: 0, bounce: 0 }, 'GamepadButton')),
    hapticActuators: [actuator],
    vibrationActuator: undefined,
  }, 'Gamepad');
  const where = [x, EYE_HEIGHT - 0.15, -0.25, 0, 0, 0, 1];
  return {
    handedness,
    targetRayMode: 'tracked-pointer',
    profiles: ['wxr-stub', 'generic-trigger-squeeze'],
    gamepad,
    // A hand and not a cursor, which is what makes this source one the skeleton can be asked about. `hand` is
    // read to answer the frame and `hand-tracking` is read to answer the feature, and they are separate
    // questions with the same object behind them.
    hand: new XRHand([x, EYE_HEIGHT - 0.15, -0.3, 0, 0, 0, 1]),
    gripSpace: new XRSpace(where),
    targetRaySpace: new XRSpace(where),
  };
}

class XRSession {
  constructor(mode, enabledFeatures) {
    this.mode = mode;
    this.environmentBlendMode = 'opaque';
    this.interactionMode = 'world-space';
    this.visibilityState = 'visible';
    this.enabledFeatures = enabledFeatures;
    this.renderState = { depthNear: 0.1, depthFar: 1000, baseLayer: null, layers: null };
    this.inputSources = [
      controller('left', -0.2),
      controller('right', 0.2),
    ];
    this.maxRenderLayers = 4;
    this.onend = null;
    this.onselect = null;
    this.onselectstart = null;
    this.onselectend = null;
    this.onsqueeze = null;
    this.onsqueezestart = null;
    this.onsqueezeend = null;
    this.oninputsourceschange = null;
    this.onvisibilitychange = null;
    this._ended = false;
    this._frame = null;
    // Every layer this session presents, which is what `XRGPUBinding` has to know: a sub-image is only ever
    // for a layer of the session that made it.
    this._layers = new Set();
    // The room's depth, which only a session granted `depth-sensing` has and only in the run that made a
    // binding. Left unset otherwise, and an unset `XRDepthUsage` is what a session that was never asked for
    // depth reports - which is the honest answer rather than a guess.
    if (GPU) {
      this.depthUsage = 'gpu-optimized';
      this.depthDataFormat = 'float32';
      this.depthType = 'raw';
      this.depthActive = true;
    }
  }

  requestReferenceSpace(type) {
    if (type === 'viewer') {
      return Promise.resolve(new XRReferenceSpace(type, null));
    }
    if (type === 'local' || type === 'local-floor' || type === 'bounded-floor' || type === 'unbounded') {
      const origin = type === 'local' ? [0, EYE_HEIGHT, 0, 0, 0, 0, 1] : [0, 0, 0, 0, 0, 0, 1];
      return Promise.resolve(new XRReferenceSpace(type, origin));
    }
    return Promise.reject(new DOMException(`there is no ${type} here`, 'NotSupportedError'));
  }

  updateRenderState(init) {
    // Between frames only, which is the rule the specification states in prose and the one call in here that
    // can throw it: `wxr-webxr` catches this, so a stub that refuses has a stub that proves the catch works.
    if (this._frame !== null) {
      throw new DOMException('updateRenderState during an animation frame', 'InvalidStateError');
    }
    if (init.depthNear !== undefined) {
      this.renderState.depthNear = init.depthNear;
    }
    if (init.depthFar !== undefined) {
      this.renderState.depthFar = init.depthFar;
    }
    if (init.layers !== undefined) {
      this.renderState.layers = init.layers;
    }
  }

  requestAnimationFrame(callback) {
    // The frame's own time is what the callback is given, and the frame is only ever handed to the callback
    // that asked for it: a frame in the hands of a page that did not ask for it is a frame the page can draw
    // with after the frame it was for has been handed back.
    return window.requestAnimationFrame((time) => {
      if (this._ended) {
        return;
      }
      this._frame = time;
      try {
        callback(time, new XRFrame(this, time));
      } finally {
        this._frame = null;
      }
    });
  }

  cancelAnimationFrame(handle) {
    window.cancelAnimationFrame(handle);
  }

  end() {
    if (this._ended) {
      return Promise.resolve();
    }
    this._ended = true;
    this.visibilityState = 'hidden';
    if (this.onend) {
      this.onend(new Event('end'));
    }
    return Promise.resolve();
  }
}

class XRFrame {
  constructor(session, time) {
    this.session = session;
    this.time = time;
  }

  /** The head, measured from whatever space was asked. */
  getViewerPose(space) {
    const near = this.session.renderState.depthNear;
    const far = this.session.renderState.depthFar;
    return new XRViewerPose(relative(headAt(this.time), space.at(this.time)), near, far);
  }

  /** Anything else: an input source's space, or a hand's joint, measured from the space it was asked in. */
  getPose(space, baseSpace) {
    if (!space || !baseSpace) {
      return null;
    }
    const where = space.at(this.time);
    return where === null ? null : new XRPose(relative(where, baseSpace.at(this.time)));
  }

  getJointPose(space, baseSpace) {
    const joint = this.session._jointOf(space);
    if (!joint) {
      return null;
    }
    const where = jointPose(joint, this.time);
    return new XRJointPose(relative(where, baseSpace.at(this.time)), joint.radius);
  }

  /**
   * Many poses at once, which is what a skeleton is asked for rather than twenty-five questions.
   *
   * The matrices go into the array at sixteen floats each, column-major, which is `XRRigidTransform.matrix`'s
   * layout - and the only layout a caller can have been written against, since a filled array is numbers and
   * nothing else. The answer is `false` when any one space had no pose, and the array is left with what could
   * be answered: a hand with three joints tracked is a hand whose array holds three matrices, and a caller
   * that reads all twenty-five has to be able to tell which were not there.
   */
  fillPoses(spaces, baseSpace, poses) {
    if (!spaces || !baseSpace || !poses) {
      throw new TypeError('fillPoses wants spaces, a base space and somewhere to put the matrices');
    }
    const origin = baseSpace.at(this.time);
    let filled = true;
    for (let index = 0; index < spaces.length; index++) {
      const space = spaces[index];
      const joint = space ? this.session._jointOf(space) : null;
      const where = space ? (joint ? jointPose(joint, this.time) : space.at(this.time)) : null;
      if (where === null || where === undefined) {
        filled = false;
        continue;
      }
      poses.set(matrixOf(relative(where, origin)), index * 16);
    }
    return filled;
  }

  /** The same question about the joints' radii, which is the other half of a hand in one call each. */
  fillJointRadii(jointSpaces, radii) {
    if (!jointSpaces || !radii) {
      throw new TypeError('fillJointRadii wants joint spaces and somewhere to put the radii');
    }
    let filled = true;
    for (let index = 0; index < jointSpaces.length; index++) {
      const joint = this.session._jointOf(jointSpaces[index]);
      if (!joint) {
        filled = false;
        continue;
      }
      radii[index] = joint.radius;
    }
    return filled;
  }
}

class XRSystem {
  isSessionSupported(mode) {
    // `inline` is real and `immersive-vr` is what this stub is for. `immersive-ar` exists only with `?gpu`,
    // because an AR session is the one that asks for `depth-sensing` and this stub's depth is a measurement
    // the binding makes: an AR session with no binding is one with nothing to measure the room with.
    return Promise.resolve(
      mode === 'inline' || mode === 'immersive-vr' || (GPU && mode === 'immersive-ar'),
    );
  }

  requestSession(mode, options = {}) {
    if (mode !== 'inline' && mode !== 'immersive-vr' && !(GPU && mode === 'immersive-ar')) {
      return Promise.reject(new DOMException('the specified session configuration is not supported', 'NotSupportedError'));
    }
    // What was asked for and is here, which is the answer `enabledFeatures` has to be: a session that claims a
    // feature it was never granted is a session whose `Features` bit is a lie.
    const wanted = [...(options.requiredFeatures || []), ...(options.optionalFeatures || [])];
    const granted = wanted.filter((feature) => STUB_FEATURES.includes(feature));
    const missing = (options.requiredFeatures || []).filter((feature) => !granted.includes(feature));
    if (missing.length) {
      return Promise.reject(new DOMException(`the runtime does not grant ${missing.join(', ')}`, 'NotSupportedError'));
    }
    return Promise.resolve(new XRSession(mode, granted));
  }
}

/**
 * What this stub can honour, which is what it puts in `enabledFeatures`.
 *
 * `webgpu` and `depth-sensing` are here only when the page asked for `?gpu`, and they are only ever granted to
 * a session that asked for them - `requestSession` below grants the intersection. Without the binding there is
 * no texture to hand over, and a session granted `webgpu` without one is a session whose `images()` promises a
 * picture the compositor will never show, which is the one thing this backend's `ImageMeta` exists to refuse to
 * do. With it, both are real: the binding makes the texture and `getDepthInformation` measures it.
 */
const STUB_FEATURES = [
  'local-floor',
  'bounded-floor',
  'hand-tracking',
  'layers',
  ...(GPU ? ['webgpu', 'depth-sensing'] : []),
];

/**
 * What the stub's projection layer hands over: one texture, one array layer per eye, as a stereo one is.
 *
 * The size is invented rather than measured, because there is no display behind this - but it is a plausible
 * per-eye size, and what the assertions are about is the *shape* of what comes back: the extent, the format,
 * the layer count, the viewport and the array layer, which is everything `image_meta` and `View::layer` read.
 */
const FRAME = { width: 1280, height: 1440, layers: 2 };
/** The one colour format this stub makes a layer in, which is one every device has. */
const COLOR_FORMAT = 'bgra8unorm';
/** The depth format `wxr-webxr` asks for by name, and the only one it will take back. */
const DEPTH_FORMAT = 'depth32float';
/** What the room's depth is in when it is measured rather than drawn into: WebXR's `float32` is `r32float`. */
const MEASURED_FORMAT = 'r32float';

/**
 * The texture usages the specification fixes the numbers of.
 *
 * Named here because `GPUTextureUsage` is a namespace object rather than a value and is not on every page, and
 * because the numbers are the specification's and do not move: a page that cannot see the namespace still wants
 * the same bits.
 */
const USAGE = typeof GPUTextureUsage === 'undefined'
  ? { RENDER_ATTACHMENT: 0x10, TEXTURE_BINDING: 0x04 }
  : GPUTextureUsage;


/**
 * The joint a `getJointPose` was asked about.
 *
 * `XRHand.get` hands back the space, so the only thing that ties a pose to a joint is that they are the same
 * object - which is how a browser does it too, since an `XRSpace` is not a value.
 */
XRSession.prototype._jointOf = function (space) {
  for (const source of this.inputSources) {
    if (!source.hand) {
      continue;
    }
    const joint = source.hand.jointOf(space);
    if (joint) {
      return joint;
    }
  }
  return null;
};

/**
 * The sub-image one view draws into, and the whole of the image path's shape.
 *
 * One texture for both eyes and a viewport the size of it, with the eye told apart by an array layer. That is
 * what a stereo projection layer *is*, and it is what `XRSubImage.viewport` means: a sub-rectangle of the layer
 * it belongs to, not of the texture - so two eyes have two viewports that happen to be the same rectangle of
 * two different layers, which is `View::viewport` and `View::layer` as the core has them.
 */
class XRGPUSubImage {
  constructor(layer, eye) {
    this._layer = layer;
    this._index = eye === 'left' ? 0 : 1;
    // `[SameObject]` in the specification, so one and the same every time it is read.
    this.viewport = { x: 0, y: 0, width: FRAME.width, height: FRAME.height };
  }

  get colorTexture() {
    return this._layer.color;
  }

  get depthStencilTexture() {
    return this._layer.depth;
  }

  // Nothing here hands one of these over, and `null` is the answer a browser with no motion vectors gives.
  get motionVectorTexture() {
    return null;
  }

  getViewDescriptor() {
    return {
      format: this._layer.format,
      dimension: '2d',
      baseMipLevel: 0,
      mipLevelCount: 1,
      // How the eye is said, and the one number this whole object is for.
      baseArrayLayer: this._index,
      arrayLayerCount: 1,
    };
  }
}

/**
 * A projection layer, and the textures behind it.
 *
 * Made through the device the page handed the binding, so the textures are the browser's and belong to the same
 * device the page's renderer has - which is the whole of what `wxr-webxr`'s importer requires to wrap one. A
 * stub that invented a texture object instead would be a stub that describes a picture the renderer then cannot
 * draw into, which is the failure this backend's `ImageMeta` exists to refuse.
 */
class XRProjectionLayer {
  constructor(binding, format, depthFormat, usage) {
    this.format = format;
    this.depthFormat = depthFormat;
    this.color = frameTexture(binding.device, format, FRAME.layers, usage);
    this.depth = depthFormat ? frameTexture(binding.device, depthFormat, FRAME.layers, usage) : null;
    // The room's depth, and not this layer's: the layer's is a buffer the app draws into, while the measured
    // one is a texture the runtime fills and takes back at the end of the frame. One layer per eye, and
    // sampled rather than rendered into, because it is read and never drawn.
    this.measured = [0, 1].map(() =>
      frameTexture(binding.device, MEASURED_FORMAT, 1, USAGE.TEXTURE_BINDING));
    this.fixedFoveation = 0;
  }
}

/** One texture of the frame's size, in the format and usage a layer asked for. */
function frameTexture(device, format, layers, usage) {
  return device.createTexture({
    size: [FRAME.width, FRAME.height, layers],
    format,
    usage,
    dimension: '2d',
  });
}

/**
 * `XRGPUBinding`, and everything about a session that has a picture.
 *
 * This replaces the browser's own when `?gpu` is asked for, and it has to: Chromium ships one behind
 * `--enable-features=WebXRWebGPUBinding`, and its constructor validates the session against the browser's own -
 * so a stub session handed to it is a `TypeError`, and a binding that never exists is a session with no images.
 * A page that asked for the stub gets the stub, all the way down.
 */
class XRGPUBinding {
  constructor(session, device) {
    if (!(session instanceof XRSession) || !device || typeof device.createTexture !== 'function') {
      throw new DOMException(
        'this binding needs a session of its own and a WebGPU device',
        'InvalidStateError',
      );
    }
    this.session = session;
    this.device = device;
  }

  get nativeProjectionScaleFactor() {
    return 1;
  }

  /** Whether the session was configured for depth, which is the same question `getDepthInformation` asks. */
  get usesDepthValues() {
    return this.session.enabledFeatures.includes('depth-sensing');
  }

  getPreferredColorFormat() {
    return COLOR_FORMAT;
  }

  createProjectionLayer(init = {}) {
    const format = init.colorFormat || COLOR_FORMAT;
    if (format !== COLOR_FORMAT) {
      throw new DOMException(`this stub makes ${COLOR_FORMAT} and no other`, 'NotSupportedError');
    }
    if (init.depthStencilFormat !== undefined && init.depthStencilFormat !== null
      && init.depthStencilFormat !== DEPTH_FORMAT) {
      throw new DOMException(`this stub makes ${DEPTH_FORMAT} depth and no other`, 'NotSupportedError');
    }
    if (this.session._ended) {
      throw new DOMException('the session has ended', 'InvalidStateError');
    }
    // The dictionary's own default, spelled out: `XRGPUProjectionLayerInit.textureUsage` is `RENDER_ATTACHMENT`,
    // and a caller that set it gets what it asked for. `wxr-webxr` leaves it alone.
    const usage = init.textureUsage === undefined ? USAGE.RENDER_ATTACHMENT : init.textureUsage;
    const layer = new XRProjectionLayer(this, format, init.depthStencilFormat || null, usage);
    this.session._layers.add(layer);
    return layer;
  }

  getViewSubImage(layer, view) {
    if (!(layer instanceof XRProjectionLayer) || !this.session._layers.has(layer)) {
      throw new DOMException("that is not this session's projection layer", 'InvalidStateError');
    }
    if (!view || (view.eye !== 'left' && view.eye !== 'right')) {
      throw new DOMException('that is not a view of this frame', 'InvalidStateError');
    }
    return new XRGPUSubImage(layer, view.eye);
  }

  /**
   * The room's depth, which is a measurement rather than a buffer.
   *
   * It refuses - and `Session::depth` answers `None` - unless the session was configured for GPU depth, which
   * is what the specification says and the only thing that makes a `None` in a session that never asked mean
   * something. `rawValueToMeters` is `1` because `float32` is metres already; a `luminance-alpha` depth would
   * have a scale here, and that is the whole of what this number is for.
   */
  getDepthInformation(view) {
    if (!this.usesDepthValues || !view || (view.eye !== 'left' && view.eye !== 'right')) {
      throw new DOMException('this session is not configured for GPU depth', 'InvalidStateError');
    }
    const layer = [...this.session._layers][0];
    if (!layer) {
      throw new DOMException('this session has no projection layer', 'InvalidStateError');
    }
    return {
      texture: layer.measured[view.eye === 'left' ? 0 : 1],
      width: FRAME.width,
      height: FRAME.height,
      rawValueToMeters: 1,
      // Nothing in this stub moves the depth, so it is where the view already is.
      normDepthBufferFromNormView: rigid([0, 0, 0, 0, 0, 0, 1]),
      transform: rigid([0, 0, 0, 0, 0, 0, 1]),
      projectionMatrix: projection(FOV[0], FOV[1], FOV[2], FOV[3], 0.1, 1000),
      getViewDescriptor: () => ({
        baseArrayLayer: view.eye === 'left' ? 0 : 1,
        mipLevelCount: 1,
        arrayLayerCount: 1,
      }),
    };
  }
}

if (GPU) {
  // Over the browser's own, for the reason in `XRGPUBinding`'s documentation above. Without `?gpu` this is
  // left alone, so a page that did not ask keeps whatever the browser really has.
  Object.defineProperty(globalThis, 'XRGPUBinding', {
    value: XRGPUBinding,
    configurable: true,
    writable: true,
  });
}

Object.defineProperty(navigator, 'xr', {
  value: new XRSystem(),
  configurable: true,
});
