// The wxr visionOS app: a SwiftUI shell whose immersive space hands the compositor's
// layer renderer to the Rust render loop in `wxr-apple`.
//
// Swift owns the app lifecycle and the immersive space; as soon as the space opens,
// `CompositorLayer` hands over a `LayerRenderer` and this file crosses into Rust with a
// single pointer. Everything after that - the frames, the device, the scene - is Rust.
//
// The import split is deliberate: `CompositorServices` is the C-backed SDK surface and
// `_CompositorServices_SwiftUI` is the SwiftUI module that carries `CompositorLayer`.
// The leading underscore is Apple's, not a private symbol of ours.

import SwiftUI
import CompositorServices
import _CompositorServices_SwiftUI

// The one symbol the Rust static library exports for this app. A `@_silgen_name`
// declaration is the whole bridge; there is no bridging header and no generated C.
//
// `layerRenderer` is the live `LayerRenderer` the compositor just made for the space,
// passed as a raw pointer because a `@_cdecl` boundary carries a pointer and nothing
// else. The call blocks until the immersive space ends and returns 0 on success or 1 on
// error (the error is logged from Rust), which is why it is called synchronously from
// the renderer closure: the closure *is* the rendering thread.
@_silgen_name("wxr_apple_run")
func wxr_apple_run(_ layerRenderer: UnsafeMutableRawPointer?) -> Int32

@main
struct WxrApp: App {
    /// The id the window's button opens and the immersive space answers to. One name in
    /// both places, declared once so the two cannot drift apart.
    static let immersiveSpaceID = "wxr"

    var body: some Scene {
        WindowGroup {
            ContentView()
        }

        ImmersiveSpace(id: WxrApp.immersiveSpaceID) {
            CompositorLayer { layerRenderer in
                // `passUnretained` is right: the compositor owns the renderer for as
                // long as the closure runs, and Rust retains it for its own duration.
                let pointer = Unmanaged.passUnretained(layerRenderer).toOpaque()
                let result = wxr_apple_run(pointer)
                if result != 0 {
                    // The Rust side has already logged the reason; this is the last
                    // trace so the failure is visible in the app's own log stream too.
                    NSLog("wxr: wxr_apple_run returned %d", result)
                }
            }
        }
        // A style has to be chosen explicitly on visionOS 1.0+. `.mixed` keeps the
        // wearer's surroundings visible around the rendered scene.
        .immersionStyle(selection: .constant(.mixed), in: .mixed)
    }
}

/// The window the app opens into: one button to open the immersive space and one to
/// close it, plus a line of status so the launch is observable without a debugger.
struct ContentView: View {
    @Environment(\.openImmersiveSpace) private var openImmersiveSpace
    @Environment(\.dismissImmersiveSpace) private var dismissImmersiveSpace
    @State private var status = "idle"

    var body: some View {
        VStack(spacing: 24) {
            Text("wxr")
                .font(.largeTitle)
            Text(status)
                .font(.footnote)
                .foregroundStyle(.secondary)

            Button("Open immersive space") {
                Task {
                    let result = await openImmersiveSpace(id: WxrApp.immersiveSpaceID)
                    switch result {
                    case .opened:
                        status = "immersive space opened"
                    case .userCancelled:
                        status = "open cancelled by the user"
                    case .error:
                        status = "opening the immersive space failed"
                    @unknown default:
                        status = "opening the immersive space returned an unknown result"
                    }
                }
            }

            Button("Close immersive space") {
                Task {
                    await dismissImmersiveSpace()
                    status = "immersive space closed"
                }
            }
        }
        .padding()
        // A headless or unattended simulator has nobody to press the button, so the
        // space is also opened once when the window appears. The button above is the
        // manual path and stays the documented one; this only makes the loop reachable
        // without a tap.
        .task {
            guard status == "idle" else { return }
            let result = await openImmersiveSpace(id: WxrApp.immersiveSpaceID)
            switch result {
            case .opened:
                status = "immersive space opened automatically"
            case .userCancelled:
                status = "automatic open cancelled"
            case .error:
                status = "automatic open failed"
            @unknown default:
                status = "automatic open returned an unknown result"
            }
        }
    }
}
