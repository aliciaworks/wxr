#!/usr/bin/env python3
"""Build the WebXR smoke page, bind it, and serve it.

Three steps, and each exists for a reason:

* `cargo build --target wasm32-unknown-unknown` is what makes a `.wasm` at all. The crate is a `cdylib`
  because that is what `wasm-bindgen` takes.
* `wasm-bindgen --target web` writes the JavaScript that the module needs - the wasm alone cannot be loaded
  by a page. The CLI has to be the version the `wasm-bindgen` crate in the lock is, because the two formats
  are tied to each other and a mismatch is a hard error rather than a warning.
* The page is written here rather than kept beside this file, so that the module's name and the page cannot
  drift apart, and so that the directory served is a build artifact.

It binds to loopback, and a browser needs a WebXR device to hand over: on a desktop that means the
Immersive Web Emulator extension, loaded with `--load-extension`. Two flags are worth knowing about:
`--enable-webgpu-developer-features` is *not* the one for WebXR (it is about WebGPU's own developer features),
and `--enable-features=WebXRWebGPUBinding` *is* - it is what puts `XRGPUBinding` in a page, which is what a
session would need to have images at all.
"""

import argparse
import functools
import http.server
import os
import shutil
import socketserver
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TARGET = "wasm32-unknown-unknown"
NAME = "wxr_webxr_smoke"

INDEX = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>wxr-webxr smoke</title>
<style>
  html, body { margin: 0; background: #111; color: #ddd; font: 13px/1.5 monospace; }
  body { white-space: pre-wrap; padding: 12px; }
</style>
</head>
<body>
<script type="module">
  import init from './__NAME__.js';
  init().catch((error) => {
    document.body.textContent += '\\ninit failed: ' + error;
    console.error(error);
  });
</script>
</body>
</html>
""".replace("__NAME__", NAME)


def build(root: str) -> str:
    """Build the crate and bind it, and answer with the directory to serve."""
    subprocess.run(
        ["cargo", "build", "-p", "wxr-webxr-smoke", "--target", TARGET, "--release"],
        cwd=root,
        check=True,
    )
    wasm = os.path.join(root, "target", TARGET, "release", f"{NAME}.wasm")
    out = os.path.join(os.path.dirname(wasm), "smoke")
    os.makedirs(out, exist_ok=True)
    bindgen = shutil.which("wasm-bindgen") or os.path.expanduser("~/.cargo/bin/wasm-bindgen")
    subprocess.run(
        [bindgen, "--target", "web", "--no-typescript", "--out-dir", out, "--out-name", NAME, wasm],
        check=True,
    )
    with open(os.path.join(out, "index.html"), "w") as page:
        page.write(INDEX)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--port", type=int, default=8800)
    parser.add_argument("--root", default=ROOT)
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()

    out = os.path.join(os.path.abspath(args.root), "target", TARGET, "release", "smoke")
    if not args.no_build:
        out = build(os.path.abspath(args.root))

    class Handler(http.server.SimpleHTTPRequestHandler):
        # `instantiateStreaming` refuses a body that is not `application/wasm`.
        extensions_map = {
            **http.server.SimpleHTTPRequestHandler.extensions_map,
            ".wasm": "application/wasm",
            ".js": "text/javascript",
        }

        def log_message(self, fmt: str, *rest) -> None:
            pass

        def do_POST(self) -> None:
            """The smoke page reports here, and this is the record.

            A page's console needs a window opened at the right moment to be read, and a driver that reads it
            is another moving part between the test and its result. The page posting its own lines means the
            result is wherever this process's output is, which for a terminal is the terminal.
            """
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length).decode("utf-8", "replace")
            for line in body.splitlines():
                print(f"smoke: {line}", flush=True)
            self.send_response(204)
            self.end_headers()

    # Bound, killed, and bound again by the next run: without this the second run finds the port in
    # `TIME_WAIT` and refuses to start, which is a thing a runner should not make anyone think about.
    socketserver.TCPServer.allow_reuse_address = True
    url = f"http://127.0.0.1:{args.port}/"
    with socketserver.TCPServer(("127.0.0.1", args.port), functools.partial(Handler, directory=out)) as server:
        print(f"wxr-webxr smoke: serving {out} at {url}", flush=True)
        server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
