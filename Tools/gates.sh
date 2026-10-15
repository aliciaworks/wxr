#!/usr/bin/env bash
#
# The gates, locally - the loop.
#
# CI runs the same list on two hosts, and the two hosts are not redundant: the workspace-wide steps (test,
# `clippy --workspace`, `doc --workspace`) are the Ubuntu host's, because `wxr-openxr` is empty on Apple and
# the things it does have - examples, doc links - want crates that only exist off it. The visionOS half needs
# an Apple host. So this runs what its host can run and says so about the rest instead of pretending.
#
# Usage: Tools/gates.sh
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

step "size (temporary rule: nothing over 500 lines)"
python3 Tools/check_file_sizes.py

# The generator fails quietly: a declaration it cannot resolve is one it leaves out, with no error at
# generation time - which is how `XRRigidTransform` once lost its constructor arguments and `XRDOMOverlayInit`
# its whole file. So the output is compared with the input here rather than taken on trust, and with the
# prototype snapshot beside the IDL for the half that asks whether a browser has the names at all. Offline
# and fast enough not to be a gate worth naming: it is the first of the README's ten checks either way.
step "bindings (wxr-webxr), against the IDL they came from"
python3 Tools/check_webxr_sys.py

step "fmt"
cargo fmt --all --check

if [[ "$(uname)" == "Darwin" ]]; then
  step "the host half - skipped: it is Ubuntu's (an empty wxr-openxr, and examples that want its crates)"
  # The one suite this host has that Ubuntu's does not: `wxr-apple` is `target_vendor = "apple"` and a Mac
  # is one, so what it tests can be linked and run here rather than only cross-checked. CI's `linked` job
  # runs the same line, with `SDKROOT` switched back from the visionOS SDK this script does not set.
  step "test (wxr-apple)"
  cargo test -p wxr-apple
else
  step "test (host)"
  cargo test
  step "clippy (host)"
  cargo clippy --workspace --all-targets -- -D warnings

  if rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then
    step "clippy (wasm32)"
    cargo clippy --workspace --all-targets --target wasm32-unknown-unknown -- -D warnings
  else
    step "clippy (wasm32) - skipped, target not installed"
  fi

  # The four targets `wxr-openxr`'s `not(wasm), not(apple)` gate is a claim about: both arms of Linux, both
  # arms of Windows, and Android, which is what a standalone headset runs. This is the *weakest* form of
  # that check and it is the only one this machine can do - CI's `native` job runs three of the four on
  # runners of their own, and only Android is a cross-compile there too. So a `clippy` and never a test, and
  # skipped rather than failed when the target is not installed: "this machine cannot ask" is not "this is
  # broken", which is what the steps around it say as well.
  cross=()
  for target in aarch64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-pc-windows-msvc aarch64-linux-android; do
    if rustup target list --installed 2>/dev/null | grep -qx "$target"; then
      cross+=("$target")
    fi
  done
  if [[ ${#cross[@]} -eq 0 ]]; then
    step "clippy (cross) - skipped, none of the four targets are installed"
  else
    for target in "${cross[@]}"; do
      step "clippy ($target)"
      cargo clippy -p wxr -p wxr-render -p wxr-openxr --all-targets --target "$target" -- -D warnings
    done
  fi

  step "doc (workspace)"
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

  # The examples are checks too, and they are the only ones that draw: `headless` runs the frame loop, `layers`
  # makes a quad against an idle session, and `rendered` imports the compositor's images and puts a picture in
  # them. All three want a runtime, and a runtime is a thing this host may not have - so the test suite above
  # skips itself when there is none, and these are asked whether there is one first rather than failing for it.
  if compgen -G "/usr/share/openxr/1/*.json" > /dev/null || [[ -e /etc/openxr/1/openxr_runtime.json ]]; then
    step "examples (wxr-openxr)"
    cargo run -q -p wxr-openxr --example headless
    cargo run -q -p wxr-openxr --example layers
    cargo run -q -p wxr-openxr --example rendered
    cargo run -q -p wxr-openxr --example room
  else
    step "examples (wxr-openxr) - skipped, this machine has no OpenXR runtime"
  fi

  # The third backend is the one that runs in a browser, so this is the only host that can run it. It needs a
  # wasm toolchain, and it needs Chromium - and a machine with neither has not disproved anything, so the
  # driver says 77 and this passes on. `serve.py` beside it builds the page, and `webxr-stub.js` beside that is
  # the WebXR the page is given, because a desktop browser has WebXR and no headset behind it.
  if rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown && command -v chromium > /dev/null; then
    step "smoke (wxr-webxr)"
    # 77 is what the driver answers when it has nothing to drive, which is a skip and not a failure - and
    # `set -e` would make the two indistinguishable, so the code is looked at rather than the exit alone.
    set +e
    PATH="$HOME/.cargo/bin:$PATH" python3 crates/wxr-webxr-smoke/drive.py
    smoke=$?
    set -e
    if [[ $smoke -eq 77 ]]; then
      step "smoke (wxr-webxr) - skipped, no browser worth driving"
    elif [[ $smoke -ne 0 ]]; then
      exit $smoke
    fi
  else
    step "smoke (wxr-webxr) - skipped, no wasm32 target or no chromium"
  fi
fi

if rustup target list --installed 2>/dev/null | grep -qx aarch64-apple-visionos; then
  step "clippy (visionOS)"
  cargo clippy -p wxr-apple --target aarch64-apple-visionos --all-targets -- -D warnings
  step "doc (visionOS)"
  RUSTDOCFLAGS="-D warnings" cargo doc -p wxr-apple --target aarch64-apple-visionos --no-deps
  step "build (visionOS) - CI's linked job, locally"
  cargo build -p wxr-apple --target aarch64-apple-visionos
else
  step "the Apple half - skipped, aarch64-apple-visionos not installed"
fi

printf '\n\033[1;32m== all gates this host has pass\033[0m\n'
