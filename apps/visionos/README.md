# wxr on visionOS

A minimal SwiftUI app that opens an `ImmersiveSpace`, hands the compositor's layer renderer to the Rust
static library, and lets the crate's own render loop draw. It is the simulator-runnable half of the Apple
backend in `crates/wxr-apple`; the renderer, the scene and the session are all Rust, and Swift is only the
app shell the platform insists on.

## What it does

* `WxrApp.swift` — the `@main` App. A `WindowGroup` with a button that opens the immersive space (and one
  that closes it), and the `ImmersiveSpace(id: "wxr")` whose `CompositorLayer` closure calls the Rust entry
  `wxr_apple_run(layerRenderer)`. The call blocks for as long as the space is open: it *is* the render
  loop. The window also opens the space once on appear, so a headless simulator can reach the loop without
  a tap; the button remains the manual path.
* `Info.plist` — the bundle keys an install needs, including `UIDeviceFamily` (7, the visionOS family),
  `NSWorldSensingUsageDescription` (world tracking) and `NSHandsTrackingUsageDescription`.
* `wxr_compositor_shim.c` — a tiny C shim for the CompositorServices and ARKit calls whose C signature
  carries a `simd` vector. See "Why a C shim" below.
* `build.sh` — builds the Rust `staticlib` for `aarch64-apple-visionos-sim`, compiles the shim, links the
  Swift executable against the archive and the frameworks, and assembles the `.app` bundle.
* `run.sh` — boots the simulator device, installs the bundle, launches it, and can stream its logs.

## Build and run

```sh
cd apps/visionos
./build.sh              # Rust staticlib + shim + Swift app
./run.sh                # boot, install, launch
./run.sh --console      # launch attached to stdout/stderr
./run.sh --log          # launch and stream the unified log
```

`build.sh --skip-rust` relinks only, for a Swift-side change. The Xcode path is taken from `DEVELOPER_DIR`
(defaulting to `/Applications/Xcode.app/Contents/Developer`), and the simulator SDK from
`$DEVELOPER_DIR/Platforms/XRSimulator.platform/Developer/SDKs/XRSimulator27.0.sdk` — `xcrun --sdk
xros-simulator` does not resolve to it. `WXR_SIM_SDK` and `WXR_SIM_UDID` override each.

The bundled device is `Apple Vision Pro`, UDID `13D6CCCE-7158-4247-AA00-756A28B9A8B7`, runtime
`com.apple.CoreSimulator.SimRuntime.xrOS-27-0`. It can be driven with no GUI: `simctl` boots it, installs,
launches, screenshots (`xcrun simctl io <udid> screenshot`) and streams logs headlessly.

## What runs in the Simulator

Verified on the visionOS 27.0 simulator: the app launches, the immersive space opens, and the render loop
draws and presents every frame.

* The window and both buttons render.
* Opening the space makes the compositor create a layer and call the `CompositorLayer` closure, which
  enters `wxr_apple_run`. The loop then runs `cp_layer_renderer_query_next_frame` → predict timing →
  query drawable → views/inputs → draw → present.
* A `sample` of the running process shows the main thread inside `entry::run`, encoding render passes into
  the drawable (`wxr_render::Renderer::draw` → `scene::Scene::draw`, MTL command encoders, blit encoders,
  `render_pass_set_vertex_buffer`) and writing the scene's uniform buffer through wgpu.
* ARKit world tracking is live in the simulator too: the device anchor comes from
  `SimVirtualHeadsetRemoteService getPose`, and content is world-locked rather than head-locked.
* A screenshot shows the compositor presenting the layer: temporarily setting the renderer's clear colour
  to magenta filled the view with magenta, and the near triangle (light blue) is drawn behind the app's
  window. With the real clear colour the space is filled with the renderer's dark clear and the triangle
  shows where the window does not cover it.

## Why a C shim (Rust changes)

The app is Swift, but driving the loop needed fixes in `crates/wxr-apple` and one line in `wxr-render`.
Each was a real failure observed in the simulator, in the order they appeared:

1. **`ffi.rs` — device limits.** `request_device` used wgpu's default limits, which exceed the simulator
   Metal device's (`max_inter_stage_shader_variables` 16 > the device's 15). `request_device` now asks for
   `adapter.limits()`, which is the compositor's own ceiling. (The error was invisible because the `log`
   crate has no logger installed in the app; `ffi.rs` now also writes the fatal error to stderr.)
2. **`arkit.rs` — a wrong pointer cast.** `as_data_provider`/`as_trackable_anchor` cast the address of the
   `Retained` *handle* instead of the object pointer it holds, so ARKit was handed a stack slot. That
   segfaulted on the first `ar_data_providers_add_data_provider`. Both now deref through `Retained`.
3. **`session.rs` — frame ordering.** `cp_frame_query_drawables` was called before
   `cp_frame_predict_timing`; the header forbids that ("Don't call this function after you call
   `cp_frame_query_drawable`") and the compositor aborted with `BUG IN CLIENT`. The timing is now predicted
   before the drawable is queried, and the update window is closed before a drawable-less frame is released.
4. **`session.rs` / `sys.rs` — the opening.** `cp_view_get_tangents` is refused for mixed reality ("please
   use `cp_drawable_compute_projection`"). The view's opening now comes from
   `cp_drawable_compute_projection` and is read back into angles by `wxr_render::angles`.
5. **`sys.rs` / the shim — the `simd` ABI.** Every C function whose signature carries a `simd` vector has
   an ABI Rust cannot express in a stable `extern`: `simd_float4x4` is returned in `q0-q3`, `simd_float4`
   in `q0`, and `simd_float2` is passed in `d0` — while a `[f32; N]` struct is returned via `x8`/`s0-s3`
   and passed in separate 32-bit registers. The original declarations compiled and linked but read
   uninitialised memory (the logged matrices were denormals). `cp_view_get_transform`,
   `cp_drawable_compute_projection` and the four ARKit transform calls now go through
   `wxr_compositor_shim.c`, which clang compiles against Apple's real headers; `sys.rs` declares only the
   shim's plain `float`/`float[16]` signatures.
6. **`scene.rs` — the near plane.** `planes()` was `0.05`, and a CompositorServices drawable refuses a near
   plane closer than `0.1` (`BUG IN CLIENT: ... less than minimum near plane of:0.100000`). It is now
   `0.1`, so the renderer's projection and the depth range told to the compositor agree.

One consequence worth knowing: `Tools/check_apple_sys.py` reads `pub fn` declarations and looks them up in
Apple's headers. The `simd` calls are no longer `pub fn` declarations here — they are the shim's, which
clang checks at compile time — so that script now checks `cp_drawable_set_device_anchor` alone from this
file. The rest of the Apple surface is unchanged.

## Known limitations

* The app is a demo shell: one scene from the crate, one window, no gestures. Where the Rust loop is what
  is being tested, that is the point.
* The rendered triangles sit behind the SwiftUI window, so on a fresh launch you see the clear colour and
  the part of the near triangle below the window. Closing the window (or opening the space fully) shows the
  whole scene.
* `session.rs` is 700 lines, over the repository's temporary 500-line rule — it was already 687 at commit
  `18d70f2`; the added lines are the reordering above. The check script already flags four other files.
