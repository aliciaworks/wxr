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

**Two numbers, and both are mechanical** - `python3 Tools/xr_coverage.py`:

| | WebXR | OpenXR | visionOS |
| --- | ---: | ---: | ---: |
| **Native API reached**, of each backend's own spec | 30% | 1% | 7% |
| **WebXR API reached**, of the whole 138-member API | 30% | 20% | 18% |

The second row is the whole WebXR API - every attribute, operation and constant its IDL declares, not just
the `Session` interface - counted member by member, and it is a *floor*. The core gives a concept its own
name (`requestReferenceSpace` is `space`) and both native backends go through the core, so a member that a
backend serves can still share no name with anything in its source. The core itself names 39 of the 138.

## The core is small on purpose

A session has a head pose, two eyes with a field of view each, an origin to measure them from - and spaces
made from that one, at a pose inside it - and an image at the end that somebody presents. That is the whole of it, and it is **WebXR's** vocabulary, because WebXR
is the only one of the three that is a specification rather than a vendor's API - OpenXR and the Apple APIs
each describe a superset in their own terms, so the smallest of the three is the one the other two reduce to.

A session is also a stream of events - a press on a source, a squeeze, the session going away - and that is
**WebXR's** vocabulary too, down to the names: `selectstart`, `selectend`, `select` and their three squeeze
siblings, because what a source *does* is not in a frame, it happens between them. A source is named by an id
the backend hands out, which WebXR does with the object itself and a Rust value cannot.

A source that is a hand is also a skeleton: twenty-five joints, `XRHandJoint` by name and a pose and a radius
each, asked for as one thing rather than a joint at a time. `InputSource::hand` is WebXR's nullable `hand` - a
source says whether there is a skeleton to ask for, which a controller and a palm-only platform both answer no
to.

Two more of WebXR's words are here for the same reason. `offset_space` is `getOffsetReferenceSpace`: a space at
a pose inside one the session handed out, which is how a scene anchors something to a place - a table, a wall, a
controller. And `set_depth_range` is `XRRenderState.depthNear` and `depthFar`, the one number a renderer and a
compositor have to agree on, because a compositor that reprojects a frame with depth cannot read the planes out
of the picture.

AR is the other half of what WebXR has: `Plane` and `Session::planes` are the surfaces a runtime has detected,
`HitTestSource` and `Session::hits` are where a ray out of a space lands on them, and `LightProbe` and
`LightEstimate` are the room's light - nine spherical-harmonic coefficients and a primary light, so a virtual
object is lit by the room rather than by a guess. Both are asked for by
`SessionMode::ImmersiveAr` - a runtime told the session is drawn over the world is the one that offers them -
and both are WebXR's alone today, because OpenXR's are extensions the crate leaves as raw pointers and Apple's
are behind a header that is not to hand.

Which of those a session actually has is `Features` - a set of bits rather than a method per capability that
errors when it is missing, the shape `wgpu` uses and WebXR's `enabledFeatures` uses. Until there was one, "this
runtime cannot do this" was said three different ways: `Error::Unsupported` where a method hands out a handle,
an empty list where it hands out many, and `None` where it hands out a thing. A session is usable without any of
them; the set is how an app knows what to offer rather than what it can survive.

Depth is the one place the specification hands over a *delivery* as well as a thing, and it is worth naming.
WebXR can give depth as bytes (`cpu-optimized`) or as a texture (`gpu-optimized`), one per session, and mirroring
that split into the core was a mistake: it is the web platform's constraint, not the concept. `Session::Depth`
is the runtime's own buffer - the same associated type `Session::Image` is for colour - and the backend picks the
delivery, which for WebXR is the texture, because a texture is what a renderer can test against. A scalar
distance is a convenience of the bytes mode and is not in the core: reading a buffer back is a `copy`, the way
`wgpu`'s is, not a second way to ask.

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

Those omissions need a way back, or they are walls rather than seams. `Session::as_backend` is it - the core's
`as_hal`, for the reason wgpu has one: a program that needs OpenXR's own eight session states, or WebXR's
`XRGPUBinding`, or a `cp_drawable`, asks for the backend's own type and gets it, and gets `None` on a
different backend. What it reaches is *not* this API and is free to change between releases; the core's
vocabulary is what is stable. It is also what lets OpenXR be the backend the native work is really done on
without letting it become the vocabulary - which stays WebXR's, the smallest of the three.

## Layout

```
crates/wxr/                the core: session, space, frame, target, layer, input. No platform code, no
                           graphics API.
crates/wxr-openxr/         the OpenXR backend, on Vulkan handles that already exist.
crates/wxr-webxr/          the WebXR backend, for wasm, with `webidl/` (the snapshot) and `src/sys/` (the
                           bindings generated from it).
crates/wxr-webxr-smoke/    the page that runs it, so that it has been run.
crates/wxr-apple/          the Apple backend: CompositorServices to present, ARKit to track. All of it is C.
crates/wxr-render/         the renderer: takes a frame and a device, draws into the images.
Tools/                     the two scripts that keep the generated bindings honest.
```

There is no `apple/` directory of Swift, and that is a finding rather than an omission - see below.

## The three backends

**`wxr-openxr`** is where the core's one architectural decision is cashed in: the renderer makes the device
and the session is told about it, through `XR_KHR_vulkan_enable`. Its `Import` is where a `VkImage` meets a
`wgpu::Texture` (`wgpu-hal`'s `Device::texture_from_raw`, which is why this workspace needs wgpu 30 - the same
30 the game it was written for is on), its `input` is OpenXR's action sets, and `examples/headless.rs` is the
proof that it reaches a real runtime - which needs a machine whose runtime is running.

The **libraries** take wgpu from the registry, and the one thing a WebXR *app* needs that upstream does not have
- `RequestAdapterOptions::xr_compatible`, without which a wgpu device can never be XR-compatible on the web - is
carried by a **fork** that this workspace patches in for its own builds only. That split is the point, and it is
in the workspace `Cargo.toml` with its history: a library that depended on a git wgpu would be a library no
other wgpu renderer could be handed a device by, because two sources are two packages. An app that wants a
WebXR session patches the fork in the same way; one that only wants OpenXR or Apple needs nothing.

**`wxr-webxr`** is the one that did not fit: WebXR's session, its reference spaces and its frames all arrive
asynchronously, which is why the core has `State::Connecting` and why a space is asked for before it exists.
It hands a renderer images when three things line up, and each is off by default somewhere: the browser has
WebXR's WebGPU binding at all, which in Chromium is behind the `webxr-webgpu-binding` flag; the session granted
`webgpu`; and the device came from an adapter requested with `xrCompatible: true`, which is the field this
workspace's wgpu fork carries. When any of them is missing the session is one with a head, two eyes, a clock
and no picture, and says so rather than pretending.

The API it speaks is **generated here rather than taken from `web-sys`**, which has WebXR's core, gates it
behind a build-wide cfg, and has none of the Layers module or the WebGPU binding. `crates/wxr-webxr/webidl/`
is the snapshot - webref's IDL for eleven specifications, and two cut out of their Bikeshed source because
webref does not carry them - and `crates/wxr-webxr/src/sys` is generated from it, committed, and checked by
`Tools/check_webxr_sys.py`. That is why nothing here needs `web_sys_unstable_apis`, and why a wrong name in
the bindings is not a thing that can compile: the generator has the names, and the only part written by hand
is the externs file and the short list in `throws.rs` of calls whose throw the specification states in prose.

Its smoke page, `crates/wxr-webxr-smoke` with a `serve.py` that builds it and serves it, is what says the
backend has been run: in a browser with the Immersive Web Emulator it gets a real session, and stops exactly
where this paragraph says it can - at visibility `Hidden`, never `Visible`, because a session does not become
visible without a layer and that browser has no WebGPU binding to make one from. The WebGPU path itself is
checked against a fake runtime instead: `Tools/xr_mock.js` in the game that consumes this is a session with a
real `GPUTexture`, and what it proves is the whole path from `XRGPUBinding` to two eyes drawn with parallax.

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
- `layer` - `Layer`, `LayerShape`, `LayerImage`. The pictures a compositor places itself, which is the one part
  of a frame that is not drawn per eye: a menu, a video, a skybox. The vocabulary is the shape both platforms
  share, and which shapes a session has is `Features::LAYER_QUAD` and its siblings.
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
cargo test                                                  # 53, on the host
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p wxr-apple --target aarch64-apple-visionos --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Each backend crate is empty outside its own target family - `wxr-openxr` off wasm, `wxr-webxr` off wasm the
other way, `wxr-apple` off Apple - so a host build tests the core and the renderer and leaves the platforms to
a cross-compiler. `.github/workflows/ci.yml` runs exactly this list.

Beyond the compiler, three things are checked, and each is checked by the one that can:

```sh
python3 Tools/check_webxr_sys.py                            # the generated bindings, against their IDL
cargo run -p wxr-openxr --example layers                    # a quad layer, against the runtime this machine has
```

The first compares the generator's output with its input - every declaration in the snapshot has a file, every
member a name - because a declaration the generator cannot resolve is one it leaves out in silence. It also
compares against a captured browser's prototypes when one is beside the IDL, which is where a name written by
hand can be caught; `Tools/capture_webxr_prototypes.js` is what produces the dump. The second is the OpenXR
half of the layer API against a real runtime: `xrWaitFrame` waits for a session that never becomes visible on a
machine with no display, so a *frame* is not available - but a layer is made against an idle session, and the
runtime takes the swapchain, hands back its images and lets go of it on release.

What none of it proves is that any of the three has drawn a frame on real hardware, because none of them has.

## What each backend reaches

Three backends, three native APIs, and a number for each that anybody can reproduce:

```sh
python3 Tools/xr_coverage.py
```

"Reached" is mechanical on purpose: the name the native API gives an item - a WebXR IDL member, an `xr*`
command, an `ar_*`/`cp_*` C function - appears in that backend's own source, outside the generated bindings.
The count is deliberately not a score. A backend that never names `XRHitTestSource` does not offer hit
testing, and saying so is checkable by grepping; and the number is a *lower bound* wherever a backend sits on
an ergonomic wrapper rather than the C names directly. `wxr-apple` calls ARKit through the generated crate
(`ar_session_t::new`, not `ar_session_create`), so the tool follows each `#[doc(alias = ...)]` back to the C
name - and OpenXR sits on the `openxr` crate, whose method names carry no such alias, which is most of why
its row reads so low.

The denominator is the whole API, extensions included: `xrCreateSession` is one of five hundred commands once
the drivers' own extensions are counted, and ARKit's C surface is six hundred functions once accessory and
room tracking are in it. Neither is what this workspace offers an app - that is ten `Features`, one
hand-written answer per backend, small enough to read in `crates/wxr/src/feature.rs`.

The first number is the one about each backend's *own*
API, which is why they differ so much: WebXR's IDL is 138 members, OpenXR's registry is 551 commands once
every vendor extension is counted, and ARKit's C surface is 618 functions once accessory and room tracking
are in it. A native item with no counterpart in the core is one no backend will ever name, so none of these
can reach 100% by design.

The second is the direction that says whether the *unification* is done, and it has two factors. The core
speaks 39 of WebXR's 138 IDL members - it is the vocabulary all three backends share rather than a mirror
of the spec - and of its own `Session` methods the backends define **94%**, **60%** and **57%**. So a
backend cannot reach the whole of WebXR by implementing methods: the word has to exist in the core first.
Widening the core and filling the methods are the two things that move this, and both are work.

## What is not decided yet

- **WebXR's images.** The import path is already in wgpu - `Device::create_texture_from_webgpu_handle`, the
  counterpart of the `texture_from_raw` the other two backends wrap their compositors' images with - and what
  is missing is one field a session needs before any of it can be used: a WebGPU-compatible session wants a
  device from an adapter requested with `xrCompatible: true`, and wgpu's public adapter options have no such
  option: it has been proposed upstream and has not landed, for being a breaking change to a public struct.
  That is the whole of what stands between this backend and a picture.
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
