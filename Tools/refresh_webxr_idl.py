#!/usr/bin/env python3
"""Refresh the WebXR bindings under `crates/wxr-webxr/src/sys`.

`web-sys` has WebXR's *core* and none of its Layers module, nor any of the WebGPU binding - and the core it
does have is behind `--cfg web_sys_unstable_apis`, a flag that would apply to every crate in any graph that
contains this one. So the whole WebXR API is generated here instead, the way wgpu generates `webgpu_sys`: from
the IDL, into Rust that is committed and pinned.

There are two inputs, and only one of them is ours:

* `crates/wxr-webxr/webidl/enabled/*.webidl` - the WebXR specifications' IDL, verbatim from webref, which is
  W3C's machine-readable IDL and is also what `web-sys` itself consumes. Pinned to `WEBREF` rather than to
  `main`, because `main` moves.
* `crates/wxr-webxr/webidl/enabled/externs.webidl` - ours. WebXR's IDL names the DOM it inherits from and the
  WebGPU and WebGL objects it hands over, and neither is WebXR's to define.

The generator is `wasm-bindgen-webidl`, the same program `web-sys` is built from. It is `publish = false`
upstream, so it has to come from a checkout of the wasm-bindgen repository:

    git clone --depth 1 --branch <tag matching this workspace's wasm-bindgen> \\
        https://github.com/wasm-bindgen/wasm-bindgen /tmp/wasm-bindgen
    cargo build --manifest-path /tmp/wasm-bindgen/Cargo.toml -p wasm-bindgen-webidl

and then point this script at the binary it built, by `--webidl-bin` or by `WASM_BINDGEN_WEBIDL`.

    python3 Tools/refresh_webxr_idl.py                 # generate from the committed IDL
    python3 Tools/refresh_webxr_idl.py --fetch         # re-fetch the IDL from webref, then generate
"""

import argparse
import os
import subprocess
import sys
import urllib.request
from pathlib import Path

# The webref commit the IDL in `webidl/` was taken from. Bump it deliberately: a spec change is a change to
# what this backend can rely on, and reading the diff is the point of pinning it.
WEBREF = "7fc4c84bdc356eab59dccb571643c8a1e1f703dd"

# The WebXR specifications that have IDL in webref. `anchors` and `raw-camera-access` are not among them -
# they are still Community Group drafts that webref has not picked up - so they are not here either.
SPECS = [
    "webxr",
    "webxrlayers",
    "webxr-webgpu-binding",
    "webxr-ar-module",
    "webxr-depth-sensing",
    "webxr-dom-overlays",
    "webxr-gamepads-module",
    "webxr-hand-input",
    "webxr-hit-test",
    "webxr-lighting-estimation",
    "webxr-plane-detection",
]

BANNER = """\
//! The WebXR API, as Rust.
//!
//! Generated from the IDL in `crates/wxr-webxr/webidl/` by `Tools/refresh_webxr_idl.py` - do not edit by
//! hand. Everything below is one `mod` and one `pub use` per interface, dictionary and enum the WebXR
//! specifications declare, under the names the generator derives from them (`XRGPUBinding` becomes
//! `XrgpuBinding`, which is why every generated `js_name` is a checkable fact rather than a guess).
//!
//! Regenerating is `python3 Tools/refresh_webxr_idl.py`; it says what it needs.

"""


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def webidl_dir() -> Path:
    return repo_root() / "crates" / "wxr-webxr" / "webidl"


def out_dir() -> Path:
    return repo_root() / "crates" / "wxr-webxr" / "src" / "sys"


def fetch() -> None:
    enabled = webidl_dir() / "enabled"
    enabled.mkdir(parents=True, exist_ok=True)
    (webidl_dir() / "unstable").mkdir(parents=True, exist_ok=True)
    for spec in SPECS:
        url = f"https://raw.githubusercontent.com/w3c/webref/{WEBREF}/ed/idl/{spec}.idl"
        with urllib.request.urlopen(url) as response:
            body = response.read()
        path = enabled / f"{spec}.webidl"
        path.write_bytes(body)
        print(f"fetched {url} -> {path.relative_to(repo_root())} ({len(body)} bytes)")


def generator(args: argparse.Namespace) -> Path:
    binary = args.webidl_bin or os.environ.get("WASM_BINDGEN_WEBIDL")
    if not binary:
        sys.exit(
            "no generator: pass --webidl-bin or set WASM_BINDGEN_WEBIDL (see this script's docstring)"
        )
    path = Path(binary)
    if not path.exists():
        sys.exit(f"no generator at {path}")
    return path


def generate(binary: Path) -> None:
    # The generator reads `<input>/enabled` and `<input>/unstable` and writes one file per interface into
    # `<output>`, which it deletes first. `--no-features` is what keeps it from writing a `#[cfg(feature =
    # "XrSession")]` per type: every one of them is in this crate, so there is nothing to gate.
    subprocess.run(
        [
            str(binary),
            str(webidl_dir()),
            str(out_dir()),
            "--no-features",
        ],
        check=True,
    )
    module = out_dir() / "mod.rs"
    module.write_text(BANNER + module.read_text())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fetch",
        action="store_true",
        help="re-fetch the IDL from webref before generating",
    )
    parser.add_argument("--webidl-bin", help="the wasm-bindgen-webidl binary to run")
    args = parser.parse_args()

    if args.fetch:
        fetch()
    generate(generator(args))
    print(f"generated {len(list(out_dir().glob('*.rs')))} files into {out_dir().relative_to(repo_root())}")


if __name__ == "__main__":
    main()
