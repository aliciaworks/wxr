#!/usr/bin/env python3
"""Drive the WebXR smoke page in a browser, and say whether it drew.

`serve.py` beside this file builds and serves the page; it does not open one, so running it ends with a URL
nobody has been told to visit. This is the other half: it serves on a port nothing else is using, launches a
browser at the page, presses the button the page is waiting for, and reads the verdict back off the record.

Four things are worth knowing about how:

* **The page is `stub.html`, and it is opened twice.** A desktop browser has WebXR and no headset behind it, so
  the page is served with `webxr-stub.js` in front of it - an XRSystem with a head, two hands and two
  controllers, made here rather than borrowed from an extension with a popup and a device catalogue. The first
  run is the plain page, which is a session with no binding and nothing to draw; the second is
  `stub.html?gpu`, the same page with the stub's `XRGPUBinding` in front of it, which is the run that has an
  image to read, a viewport and an array layer per eye, and a measured depth. Both have to say `done`, and a
  failure in either is this driver's failure. `index.html` is the same page against whatever the browser really
  has, for a machine that has something.
* **The verdict comes back over `/log`**, the same channel the run's own log lines use. The page has exactly
  two of them, `done` and `FAILED`. Reading them off the page instead would mean a driver that could be
  reading a page that had not started yet, and a DOM that had to be asked would be a third place for the
  answer to live when the server is already the first.
* **The button is pressed, not clicked by script.** `requestSession` is one of the calls the specification
  reserves for a user gesture, so a page that asks on load is refused with `SecurityError` - and a synthetic
  `.click()` is not a gesture either. What is one is real input, so this speaks the DevTools protocol's
  `Input` domain and dispatches a mouse press at the button. The protocol is spoken over the browser's
  `--remote-debugging-pipe`, which is NUL-terminated JSON on file descriptors 3 and 4: no WebSocket to
  hand-roll, no client library to have, and nothing listening on a port.
* **No browser is a skip, not a failure.** A machine with no Chromium has not disproved the backend, so that
  exits 77, which is what a skip has always been, and it can be pointed at with `WXR_BROWSER`.
"""

# The `X | None` annotations below are `type.__or__`, which Python answered only from 3.10, and `gates.sh` runs
# this file with whatever `python3` is on the PATH - which on a Mac without Homebrew's is /usr/bin/python3, still
# 3.9. Deferring their evaluation is what `__future__` is for, and nothing here reads an annotation at runtime.
from __future__ import annotations

import argparse
import functools
import http.server
import json
import os
import select
import shutil
import socketserver
import subprocess
import sys
import tempfile
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import serve  # noqa: E402  - beside this file, and its `build` is the half this does not repeat.

# The browser to look for, in the order a Linux desktop is likely to have one. `WXR_BROWSER` overrides.
BROWSERS = ("chromium", "google-chrome", "chromium-browser", "chrome")

# What the page says when it is waiting, which is when it is safe to press.
WAITING = "waiting for a click to enter XR"
DONE = "done"
FAILED = "FAILED"

# The two pages, and the two answers a session has to give: nothing to draw, and something to draw. The
# question mark is not decoration - `webxr-stub.js` reads it and puts an `XRGPUBinding` in the page only for
# the second one.
PAGES = ("stub.html", "stub.html?gpu")


def find_browser(explicit: str | None) -> str | None:
    """The browser to drive, or `None` if this machine has none worth driving."""
    if explicit:
        return explicit
    for name in BROWSERS:
        if shutil.which(name):
            return name
    return None


class Record(http.server.SimpleHTTPRequestHandler):
    """The page's own log, and the verdict at the end of it.

    `serve.py` prints these; this one also keeps them, because a driver needs to say what it saw when the
    answer was not the one it was waiting for. Printing alone would make a timeout a silence.
    """

    lines: list[str] = []
    verdict = threading.Event()
    waiting = threading.Event()
    lock = threading.Lock()

    @classmethod
    def reset(cls) -> None:
        """Forget the last run's lines, which are the last run's.

        Two pages are driven one after the other and both write here, so a verdict left over from the first
        would be the second run's answer read off the first run's record.
        """
        with cls.lock:
            cls.lines = []
        cls.verdict.clear()
        cls.waiting.clear()

    def __init__(self, *args, directory=None, **kwargs):
        """A request handler that serves the built page and records what the page posts."""
        super().__init__(*args, directory=directory, **kwargs)

    def do_POST(self) -> None:  # noqa: N802  - the name the base class has.
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode("utf-8", "replace")
        for line in body.splitlines():
            with Record.lock:
                Record.lines.append(line)
            print(f"smoke: {line}", flush=True)
            if line == WAITING:
                Record.waiting.set()
            if line == DONE or line.startswith(FAILED):
                Record.verdict.set()
        self.send_response(204)
        self.end_headers()

    def log_message(self, fmt: str, *rest) -> None:
        """Silent, because a page's GETs are not what this is about and the log lines already say plenty."""
        pass


class Pipe:
    """The DevTools protocol over `--remote-debugging-pipe`: NUL-terminated JSON on fds 3 and 4.

    A browser started that way answers on a pipe instead of a port, which is the difference between a driver
    with no moving parts and one that has to find a free port, speak WebSocket framing, and shut a server
    down again. The protocol itself is the same JSON either way.
    """

    def __init__(self, reader: int, writer: int):
        self.reader = reader
        self.writer = writer
        self.buffer = b""
        self.next_id = 0

    def send(self, message: dict) -> None:
        os.write(self.writer, json.dumps(message).encode("utf-8") + b"\0")

    def take(self, deadline: float) -> dict | None:
        """The next complete message, or `None` if the deadline passed first."""
        while b"\0" not in self.buffer:
            if time.monotonic() > deadline:
                return None
            ready, _, _ = select.select([self.reader], [], [], 0.25)
            if not ready:
                continue
            chunk = os.read(self.reader, 1 << 16)
            if not chunk:
                return None
            self.buffer += chunk
        raw, self.buffer = self.buffer.split(b"\0", 1)
        return json.loads(raw.decode("utf-8", "replace"))

    def call(self, method: str, params: dict, session: str | None, deadline: float) -> dict | None:
        """One command and its answer, skipping anything else the browser said in between."""
        self.next_id += 1
        wanted = self.next_id
        message: dict = {"id": wanted, "method": method, "params": params}
        if session:
            message["sessionId"] = session
        self.send(message)
        while True:
            answer = self.take(deadline)
            if answer is None:
                return None
            if answer.get("id") == wanted:
                return answer


def open_page(browser: str, url: str) -> tuple[subprocess.Popen, Pipe, str]:
    """A browser at the page, and the protocol handle on it.

    The pipes are wired onto fds 3 and 4 in the child, which is the only pair the browser looks at, and the
    server is not started until after the fork - a `preexec_fn` running while another thread holds a lock is
    how a Python program deadlocks in a way that says nothing about what it was doing.
    """
    to_browser_r, to_browser_w = os.pipe()
    from_browser_r, from_browser_w = os.pipe()

    def wire() -> None:
        os.dup2(to_browser_r, 3)
        os.dup2(from_browser_w, 4)

    flags = [
        # One target, and `=new` rather than the bare flag because which one the bare flag means has moved
        # about across releases - and this one is the headless that behaves like the browser.
        "--headless=new",
        "--no-first-run",
        "--no-default-browser-check",
        # A profile that is thrown away, so a run cannot be helped or hindered by whatever the last left.
        f"--user-data-dir={tempfile.mkdtemp(prefix='wxr-webxr-smoke-')}",
        # `XRGPUBinding` is a developer feature in Chromium. Without the flag the page has WebXR and no way to
        # hand an image over, which is a session that draws nothing and says nothing about why.
        "--enable-features=WebXR,WebXRWebGPUBinding",
        # WebGPU on a machine with no GPU, which is most of CI.
        "--enable-unsafe-webgpu",
        "--use-angle=swiftshader",
        # `/dev/shm` is small in a container and Chromium's default allocation of it is not.
        "--disable-dev-shm-usage",
        # Background chatter, which is noise between a run and the lines it is about.
        "--disable-background-networking",
        "--remote-debugging-pipe",
        url,
    ]
    print(f"smoke: driving {browser} at {url}", flush=True)
    process = subprocess.Popen(
        [browser, *flags],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        pass_fds=(3, 4),
        preexec_fn=wire,
    )
    os.close(to_browser_r)
    os.close(from_browser_w)
    pipe = Pipe(from_browser_r, to_browser_w)

    deadline = time.monotonic() + 60
    while True:
        targets = pipe.call("Target.getTargets", {}, None, deadline)
        if targets is None:
            raise RuntimeError("the browser never answered for its targets")
        pages = [t for t in targets["result"]["targetInfos"] if t["type"] == "page"]
        if pages:
            break
        time.sleep(0.25)
    attached = pipe.call(
        "Target.attachToTarget", {"targetId": pages[0]["targetId"], "flatten": True}, None, deadline
    )
    if attached is None or "result" not in attached:
        raise RuntimeError("the browser refused to be attached to")
    return process, pipe, attached["result"]["sessionId"]


def press(pipe: Pipe, session: str, x: float, y: float) -> None:
    """A press and a release at a point, which is what a browser counts as a gesture.

    `Runtime.evaluate` with `userGesture` is the other way, and it is a worse one: it says the gesture
    happened rather than making one, so what it proves is that the browser believes a flag.
    """
    deadline = time.monotonic() + 30
    common = {"x": x, "y": y, "button": "left", "clickCount": 1, "pointerType": "mouse"}
    pipe.call("Input.dispatchMouseEvent", {**common, "type": "mousePressed", "buttons": 1}, session, deadline)
    pipe.call("Input.dispatchMouseEvent", {**common, "type": "mouseReleased", "buttons": 0}, session, deadline)


def drive(browser: str, url: str, seconds: float) -> int:
    """Open the page, press its button, wait for its verdict, and answer with whether it drew."""
    process = None
    try:
        process, pipe, session = open_page(browser, url)
        pipe.call("Runtime.enable", {}, session, time.monotonic() + 30)

        # The page says when it is waiting, and only a page that is waiting can be pressed: a click at the
        # moment the button is drawn is a click at coordinates the page has not laid out yet.
        if not Record.waiting.wait(seconds):
            print("smoke: the page never said it was waiting", flush=True)
            return 1

        where = pipe.call(
            "Runtime.evaluate",
            {
                "expression": "(function () {"
                "  var b = document.getElementById('enter');"
                "  var r = b.getBoundingClientRect();"
                "  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };"
                "})()",
                "returnByValue": True,
            },
            session,
            time.monotonic() + 30,
        )
        value = where["result"]["result"].get("value") if where else None
        if not value:
            print("smoke: no button to press", flush=True)
            return 1
        print(f"smoke: pressing the button at ({value['x']:.0f}, {value['y']:.0f})", flush=True)
        press(pipe, session, value["x"], value["y"])

        Record.verdict.wait(seconds)
        with Record.lock:
            lines = list(Record.lines)
        if not Record.verdict.is_set():
            print(f"smoke: no verdict after {seconds:.0f}s", flush=True)
            return 1
        return 0 if any(line == DONE for line in lines) else 1
    except RuntimeError as error:
        print(f"smoke: {error}", flush=True)
        return 1
    finally:
        if process is not None:
            # By pid and never by name: this browser is not the only one this machine may have, and a driver
            # that killed the ones it did not start is a driver with a grudge.
            process.terminate()
            try:
                process.wait(15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(15)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--browser", default=os.environ.get("WXR_BROWSER"), help="what to launch")
    parser.add_argument("--root", default=serve.ROOT)
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()

    browser = find_browser(args.browser)
    if browser is None:
        print("smoke: no Chromium to drive; set WXR_BROWSER", flush=True)
        return 77  # EX_SKIP: a machine with no browser has not failed, it has skipped.

    root = os.path.abspath(args.root)
    out = (
        os.path.join(root, serve.TARGET, "release", "smoke")
        if args.no_build
        else serve.build(root)
    )

    # Bound before the browser is started and served after it, so that the fork above happens while this
    # thread is the only one there is.
    socketserver.TCPServer.allow_reuse_address = True
    server = socketserver.TCPServer(("127.0.0.1", 0), functools.partial(Record, directory=out))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        origin = f"http://127.0.0.1:{server.server_address[1]}"
        # Two runs of one page, and the query string is the whole of the difference - see the note at the top.
        # A skip is a skip for both: a machine with no browser has not failed either of them.
        failures = 0
        for page in PAGES:
            Record.reset()
            print(f"smoke: --- {page}", flush=True)
            if drive(browser, f"{origin}/{page}", args.timeout) != 0:
                failures += 1
        return failures
    finally:
        server.shutdown()


if __name__ == "__main__":
    raise SystemExit(main())
