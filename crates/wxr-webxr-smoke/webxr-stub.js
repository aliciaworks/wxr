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
// that has no images to give. It has no `XRGPUBinding` that means anything, so a session granted `webgpu`
// would be a session granted a feature it cannot honour: it is not in `enabledFeatures`, and a session like
// that is what `Presentation::Composited` with no pictures *is*. That is the honest answer and it is also the
// branch this backend has never had anything to test.
//
// It is loaded by `stub.html` and not by `index.html`, which is the whole of how it is opted into: a page that
// asked for the stub gets it, and a page that did not gets whatever the browser really has.

'use strict';

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
}

class XRSystem {
  isSessionSupported(mode) {
    // `inline` is real and `immersive-vr` is what this stub is for. `immersive-ar` is not, and a stub that
    // claimed it would be a stub whose session had a world it had no way to make.
    return Promise.resolve(mode === 'inline' || mode === 'immersive-vr');
  }

  requestSession(mode, options = {}) {
    if (mode !== 'inline' && mode !== 'immersive-vr') {
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
 * `webgpu` is deliberately absent. There is no binding here that could hand over a texture, and a session
 * granted `webgpu` without one is a session whose `images()` promises a picture the compositor will never
 * show - which is the one thing this backend's `ImageMeta` exists to refuse to do.
 */
const STUB_FEATURES = [
  'local-floor',
  'bounded-floor',
  'hand-tracking',
  'layers',
];

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

Object.defineProperty(navigator, 'xr', {
  value: new XRSystem(),
  configurable: true,
});
