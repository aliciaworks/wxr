# wxr

An XR core. One vocabulary for a session, and a backend per platform that fills it in:

```
                    wxr                     the session, the views, the poses, the images
                     │
        ┌────────────┼────────────┐
        │            │            │
     OpenXR        WebXR         ARKit            who actually knows where the head is
        │            │            │
        └────────────┼────────────┘
                     │
                   wxr-render                  the picture, drawn once
                     │
                   wgpu
```

Nothing above the renderer names a graphics API, and nothing above `wgpu` creates a device.

## The core is small on purpose

A session has a head pose, two eyes with a field of view each, an origin to measure them from, and an image
at the end that somebody presents. That is the whole of it, and it is **WebXR's** vocabulary, because WebXR
is the only one of the three that is a specification rather than a vendor's API - OpenXR and RealityKit each
describe a superset in their own terms, so the smallest of the three is the one that the other two reduce to.

Three things the core deliberately does not know:

**What a graphics API is.** `Session::Image` is an associated type. OpenXR's images are Vulkan or D3D12
handles, WebXR's only exist once they are bound to a device, and RealityKit has none at all. A core that
named one of them would make the other two second-class citizens, so the core carries images and never looks
inside one. The code that knows how to wrap one is written per backend, next to the backend.

**Who creates the device.** The renderer does, and the backend is told afterwards. This is the inversion
that the whole design turns on, and it is worth being explicit about why, because the natural order - the
runtime hands out a device and the renderer adopts it - is how an XR layer ends up dictating the graphics
API to everything above it.

- **OpenXR** has two ways to do this. `XR_KHR_vulkan_enable2` has the runtime create `VkInstance` and
  `VkDevice` for you; `XR_KHR_vulkan_enable` takes an instance and a device that already exist and only
  needs to be told which physical device and queue you made. The first is the trap: it looks like the polite
  option, and it makes the XR layer the owner of the device - which means every other graphics decision has
  to be made through it afterwards. This core is built for the second.
- **WebXR** is already the second: the page creates the `GPUDevice`, and the session hands back sub-images
  bound to it.
- **RealityKit** has no device to offer, which is the next point.

**That the app renders at all.** `Presentation` is either `Composited` - the app draws into images the
compositor owns, which is OpenXR and WebXR - or `Scene`, where the platform draws the scene the app
describes and there is no image to draw into. That is RealityKit, and it is not an edge case; it is a whole
platform. A core that assumed the first could only express two thirds of its own diagram.

## Layout

```
crates/wxr/            the core: session, space, frame, target. No platform code, no graphics API.
crates/wxr-openxr/     the OpenXR backend. Vulkan first, D3D12 after.
crates/wxr-webxr/      the WebXR backend, for wasm.
crates/wxr-arkit/      the Apple backend. RealityKit, so `Presentation::Scene`.
crates/wxr-render/     the renderer: takes a frame and a device, draws into the images.
apple/                 the Swift half of the Apple backend, which is where RealityKit has to live.
```

`wxr-openxr` is the first backend, and it is where the core's one architectural decision is cashed in: the
renderer makes the device and the session is told about it, through `XR_KHR_vulkan_enable`. It compiles and
reaches a real runtime; `examples/headless.rs` is the proof, and it needs a machine whose runtime is
running.

## What `wxr` is now

The model, the seam, and a mock:

- `space` - `Pose`, `ReferenceSpace`, `SpaceKind`, and the arithmetic that turns a hand in a shoulder into a
  hand in a room.
- `frame` - `View`, `Eye`, `FieldOfView`, `Frame`, `FrameState`. Asymmetric fields of view, because a
  headset lens is not centred on its panel.
- `target` - `ImageMeta`, `ColorFormat`, `Extent2d`. The *shape* of the images, not the images.
- `session` - the `Session` and `Backend` traits, `State`, `Event`, `Presentation`, `Error`.
- `mock` - a backend with no runtime behind it, so a renderer can be written against a session before any
  backend exists, and the core can be tested without a headset.

```sh
cargo test -p wxr
```

## What is not decided yet

- **The image seam.** `Session::Image` is an associated type, which keeps the core honest, but the exact
  shape of "here is the image, wrap it" is not settled until the first real backend is written against it.
  A `VkImage` and its memory, an acquired `XRGPUSubImage` and the device it is bound to, and a
  `CAMetalLayer`'s texture are three different things, and the trait should be whatever all three can be
  without a cast.
- **Input.** Sessions have inputs - poses, buttons, hands - and none of it is here yet. It belongs in the
  core, but its shape should follow whatever OpenXR and WebXR actually agree about rather than being guessed
  at now.
- **Where the renderer's device meets OpenXR.** The renderer creates the device and the OpenXR session is
  told about it, which means the backend needs a step that takes the handles. That step is a trait, and
  which side it is declared on is the next decision.
