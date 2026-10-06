# wxr

experimental project, co-authored by deepseek-v4.1-flash

An XR core. One vocabulary for a session, and a backend per platform that fills it in:

```
                     wxr                    the session, the views, the poses, the images
                      │
         ┌────────────┼────────────┐
         │            │            │
      OpenXR        WebXR      CompositorServices       who knows where the head is
         │            │        + ARKit (both are C)
         └────────────┼────────────┘
                      │
                   wxr-render               the picture, drawn once
                      │
                     wgpu
```

Nothing above the renderer names a graphics API, and the XR layer never makes a device: where the platform
owns one, wgpu is made to adopt it.

## The core is small on purpose

A session has a head pose, two eyes with a field of view each, an origin to measure them from, and an image
at the end that somebody presents. That is the whole of it, and it is **WebXR's** vocabulary, because WebXR
is the only one of the three that is a specification rather than a vendor's API - OpenXR and the Apple APIs
each describe a superset in their own terms, so the smallest of the three is the one the other two reduce to.

Three things the core deliberately does not know:

**What a graphics API is.** `Session::Image` is an associated type. OpenXR's images are Vulkan or D3D12
handles, WebXR's only exist once they are bound to a device, and a compositor's are `MTLTexture`s. A core
that named one of them would make the other two second-class citizens, so the core carries images and never
looks inside one. The code that knows how to wrap one is written per backend, next to the backend.

**Who creates the device.** The renderer does, and the backend is told afterwards. This is the inversion the
whole design turns on, and it is worth being explicit about why, because the natural order - the runtime
hands out a device and the renderer adopts it - is how an XR layer ends up dictating the graphics API to
everything above it.

- **OpenXR** has two ways to do this. `XR_KHR_vulkan_enable2` has the runtime create `VkInstance` and
  `VkDevice` for you; `XR_KHR_vulkan_enable` takes an instance and a device that already exist and only needs
  to be told which physical device and queue you made. The first is the trap: it looks like the polite
  option, and it makes the XR layer the owner of the device, which means every other graphics decision has to
  be made through it afterwards. This core is built for the second.
- **WebXR** is already the second: the page creates the `GPUDevice`, and the session hands back sub-images
  bound to it.
- **CompositorServices** is the one platform that contradicts it, and honestly: the compositor owns the
  `MTLDevice`, because the textures it hands out belong to one. So the app makes wgpu adopt that device and
  hands the session the queue it draws on - which is what `Backend::Device` is for, and why it is an
  associated type rather than a wgpu device by name.

**That the app renders at all.** `Presentation` is either `Composited` - the app draws into images the
compositor owns, which is OpenXR, WebXR and CompositorServices - or `Scene`, where the platform draws the
scene the app describes and there is no image to draw into. That is RealityKit, and it is not an edge case;
it is a whole platform. A core that assumed the first could only express two thirds of its own diagram. No
backend here takes the `Scene` arm: RealityKit would mean the platform draws and *our* renderer does not.

## Layout

```
crates/wxr/                the core: session, space, frame, target, input. No platform code, no graphics API.
crates/wxr-openxr/         the OpenXR backend, on Vulkan handles that already exist.
crates/wxr-webxr/          the WebXR backend, for wasm.
crates/wxr-webxr-smoke/    the page that runs it, so that it has been run.
crates/wxr-apple/          the Apple backend: CompositorServices to present, ARKit to track. All of it is C.
crates/wxr-render/         the renderer: takes a frame and a device, draws into the images.
```

There is no `apple/` directory of Swift, and that is a finding rather than an omission - see below.

## The three backends

**`wxr-openxr`** is where the core's one architectural decision is cashed in: the renderer makes the device
and the session is told about it, through `XR_KHR_vulkan_enable`. Its `Import` is where a `VkImage` meets a
`wgpu::Texture` (`wgpu-hal`'s `Device::texture_from_raw`, which is why this workspace needs wgpu 30 - the same
30 the game it was written for is on), its `input` is OpenXR's action sets, and `examples/headless.rs` is the
proof that it reaches a real runtime - which needs a machine whose runtime is running.

That wgpu comes from a **fork**, pinned to a commit: it carries one field upstream does not have,
`RequestAdapterOptions::xr_compatible`, without which a wgpu device can never be XR-compatible on the web. The
history and the removal condition are in the workspace `Cargo.toml`, and the field itself is one line of
`wgpu-types` plus two of the browser backend.

**`wxr-webxr`** is the one that did not fit: WebXR's session, its reference spaces and its frames all arrive
asynchronously, which is why the core has `State::Connecting` and why a space is asked for before it exists.
It also cannot give a renderer any images yet - WebXR's WebGPU binding (`XRGPUBinding`) is not in `web-sys`,
and in a browser it is behind the `webxr-webgpu-binding` flag, which does work on Linux although the
announcement named Windows and Android - so it hands over the head, the eyes and the timing and says so
plainly. It has a smoke page of its own, `crates/wxr-webxr-smoke` with a `serve.py` that builds it and serves
it, because a backend nobody has run is a backend nobody has seen work: in a browser with the Immersive Web
Emulator it gets a real session, and stops exactly where this paragraph says it must - with visibility
`Hidden`, never `Visible`, because a session does not become visible without a base layer and there is no WebGPU
binding to make one from in that browser.

**`wxr-apple`** was going to need a Swift shim, and does not. Two things were learned from Apple's own
documentation rather than assumed:

- `cp_view_get_transform` and `cp_view_get_tangents` are C functions that `objc2-compositor-services` does
  not bind, so `sys` declares them from Apple's C guide - which calls both. The eyes do not need Swift.
- ARKit's visionOS *Swift* API has no Objective-C presence, which is true, and does not mean ARKit is
  unreachable: "ARKit in visionOS C API" is a complete second surface for C and C++ engines. `arkit` is that
  session, its world tracking and its hand tracking.

What is left for Swift is the app's own entry - an `ImmersiveSpace` whose `CompositorLayer` closure hands the
layer renderer over - which is three lines, belongs to the app, and translates nothing. The rest is C: the
frame loop, the drawable's textures, each view's texture map, and a present encoded into a command buffer of
its own on wgpu's queue.

One platform fact comes with it: a visionOS drawable's depth is **reverse-Z**, so `Depth::Reverse` is what a
pass drawn into one uses, and the session says so.

## What `wxr` is now

The model, the seam, and a mock:

- `space` - `Pose`, `ReferenceSpace`, `SpaceKind`, and the arithmetic that turns a hand in a shoulder into a
  hand in a room.
- `frame` - `View`, `Eye`, `FieldOfView`, `Frame`, `FrameState`. Asymmetric fields of view, because a
  headset lens is not centred on its panel.
- `target` - `ImageMeta`, `ColorFormat`, `Extent2d`. The *shape* of the images, not the images.
- `session` - the `Session` and `Backend` traits, `State`, `Event`, `Presentation`, `Error`.
- `input` - `InputSource`, `Buttons`, `Axes`. One entry per hand, with a grip pose and an aim pose, because a
  controller is one thing with two places on it. The buttons are only what all three platforms have, and on a
  platform with none of them - a hand - they are empty.
- `mock` - a backend with no runtime behind it, so a renderer can be written against a session before any
  backend exists, and the core can be tested without a headset.

## Verifying it

There is no headset here, so verification is what compiles and what is tested:

```sh
cargo fmt --all --check
cargo test                                                  # 27, on the host
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p wxr-apple --target aarch64-apple-visionos --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Each backend crate is empty outside its own target family - `wxr-openxr` off wasm, `wxr-webxr` off wasm the
other way, `wxr-apple` off Apple - so a host build tests the core and the renderer and leaves the platforms to
a cross-compiler. `.github/workflows/ci.yml` runs exactly this list. What none of it proves is that any of the
three has drawn a frame on real hardware, because none of them has.

## What is not decided yet

- **WebXR's images.** The import path is already in wgpu - `Device::create_texture_from_webgpu_handle`, the
  counterpart of the `texture_from_raw` the other two backends wrap their compositors' images with - and what
  is missing is one field a session needs before any of it can be used: a WebGPU-compatible session wants a
  device from an adapter requested with `xrCompatible: true`, and wgpu's public adapter options have no such
  option, because the pull request that would have added it
  ([#9350](https://github.com/gfx-rs/wgpu/pull/9350)) dropped it as a breaking change after agreeing the shape
  of it ([#8329](https://github.com/gfx-rs/wgpu/issues/8329)). That is the whole of what stands between this
  backend and a picture.
- **The Apple app.** The Rust half of its entry point is `wxr_apple::entry::run`: connect, pick a space, and
  run frames until the space closes, with a callback for the app's own work. What is left is the SwiftUI file
  around it, its `Info.plist` - `NSWorldSensingUsageDescription` is what ARKit refuses without - and the
  app's choice of how to get the compositor's device into wgpu, which `AppleBackend::device` documents.
- **A real scene.** `wxr-render` draws two triangles with a depth buffer and a fixed near and far: enough
  for `Depth` to be load-bearing - the nearer one wins whichever order they are drawn in, in both conventions,
  and that is read back off the GPU in a test - and not a scene. Lighting, textures, instancing and a tone map
  are a renderer's business and this one is a seam. What is missing in the same tier is *submitting* the depth
  to the compositor: all three runtimes can take it and none of this does.
- **Input beyond the intersection.** Gestures, the hand skeleton, foveation and haptics are all real platform
  features that this core says nothing about on purpose. The day one of them is needed, it is one `cfg` away
  - and the seam it should come through is worth choosing then rather than now.
