#!/usr/bin/env python3
"""How much of each native API its backend actually reaches.

Each crate here is a backend that consumes one native API: `wxr-webxr` drives WebXR, `wxr-openxr` drives
OpenXR, `wxr-apple` drives ARKit and CompositorServices on visionOS. "Implemented" below means exactly one
thing and it is deliberately mechanical: the name the native API gives an item - a WebXR IDL member, an
`xr*` OpenXR command, an `ar_*`/`cp_*` C function - appears in that backend's own source, outside the
generated bindings. A backend that never names `XRHitTestSource` does not offer hit testing, whatever else
it does, and a count that says so is one a reader can check by grepping.

What this is *not* is a count of what the core offers an app - that is [`wxr::Features`], ten capabilities
with a hand-written answer per backend, and it is small enough to read. This is the other side of the same
question: of the API a backend could be reaching, how much is it reaching yet.

Usage:
    python3 Tools/xr_coverage.py            # uses a cached OpenXR registry, fetching it the first time
    python3 Tools/xr_coverage.py --no-network
"""

import argparse
import re
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The OpenXR registry, which is the spec: every command, structure and enum the API has. Cached because
# it is fetched, and pinned by being named here rather than tracking `main`.
XR_XML_URL = (
    "https://raw.githubusercontent.com/KhronosGroup/OpenXR-SDK/main/"
    "specification/registry/xr.xml"
)
XR_XML_CACHE = ROOT / "Tools" / ".cache" / "xr.xml"

# The visionOS SDK the Apple backend is built against. `check_apple_sys.py` reads the same trees.
def find_sdk():
    candidates = [Path.home() / "Developer" / "experimental" / "XROS.sdk"]
    try:
        import subprocess

        out = subprocess.run(
            ["xcrun", "--sdk", "xros", "--show-sdk-path"],
            capture_output=True,
            text=True,
        )
        if out.returncode == 0:
            candidates.append(Path(out.stdout.strip()))
    except FileNotFoundError:
        pass
    for app in sorted(Path("/Applications").glob("Xcode*.app")):
        candidates.append(
            app / "Contents/Developer/Platforms/XROS.platform/Developer/SDKs/XROS.sdk"
        )
    return next((p for p in candidates if p.is_dir()), None)


SDK = find_sdk()
APPLE_FRAMEWORKS = {
    "ARKit": "System/Library/Frameworks/ARKit.framework/Headers",
    "CompositorServices": "System/Library/Frameworks/CompositorServices.framework/Headers",
}


def source_of(crate: str, skip_generated=True) -> str:
    """A backend's own source, with the machine-written parts left out."""
    root = ROOT / "crates" / crate / "src"
    parts = []
    for path in sorted(root.rglob("*.rs")):
        if skip_generated and ("generated" in path.parts or path.parts[-2:-1] == ("sys",)):
            continue
        parts.append(path.read_text(errors="ignore"))
    return "\n".join(parts)


def names_in(text: str, pattern: str) -> set:
    return {m.group(1) for m in re.finditer(pattern, text)}


def webxr_items() -> set:
    """Every member the WebXR IDL declares: attributes, operations and constants.

    The IDL in `crates/wxr-webxr/webidl/enabled/` is the pinned spec, checked in, so there is nothing to
    fetch and nothing to guess at.
    """
    items = set()
    idl = ROOT / "crates" / "wxr-webxr" / "webidl" / "enabled"
    for path in sorted(idl.glob("*.webidl")):
        text = re.sub(r"//.*", "", path.read_text(errors="ignore"))
        items |= names_in(text, r"(?:readonly\s+)?attribute\s+[\w<>?,\[\]\s]+?\s(\w+)\s*;")
        items |= names_in(text, r"^\s*(?:[\w<>?,\[\]\s]+?)\s(\w+)\s*\([^;]*\)\s*;")
        items |= names_in(text, r"\bconst\s+[\w<>?,\[\]\s]+?\s(\w+)\s*=")
    return items


def openxr_items(fetch: bool) -> set:
    """Every `xr*` command the OpenXR registry declares."""
    if not XR_XML_CACHE.exists():
        if not fetch:
            return set()
        XR_XML_CACHE.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(XR_XML_URL, timeout=60) as r:
            XR_XML_CACHE.write_bytes(r.read())
    text = XR_XML_CACHE.read_text(errors="ignore")
    return names_in(text, r'<command\s+name="(xr\w+)"')


def visionos_items() -> set:
    """Every `ar_*`/`cp_*` C function the visionOS SDK declares in the two frameworks this backend uses."""
    items = set()
    if SDK is None:
        return items
    for framework, rel in APPLE_FRAMEWORKS.items():
        headers = SDK / rel
        if not headers.is_dir():
            continue
        for path in sorted(headers.glob("*.h")):
            text = path.read_text(errors="ignore")
            items |= names_in(text, r"\b(ar_\w+)\s*\(")
            items |= names_in(text, r"\b(cp_\w+)\s*\(")
    return items


def report(title: str, total: set, source: str) -> tuple:
    hit = {n for n in total if re.search(rf"\b{re.escape(n)}\b", source)}
    pct = (100.0 * len(hit) / len(total)) if total else 0.0
    print(f"\n## {title}: {len(hit)} / {len(total)}  ({pct:.0f}%)")
    missing = sorted(total - hit)
    if missing:
        print(f"   not reached yet ({len(missing)}): " + ", ".join(missing[:24]) +
              (" ..." if len(missing) > 24 else ""))
    return len(hit), len(total)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-network", action="store_true", help="do not fetch the OpenXR registry")
    args = ap.parse_args()

    print("# Native API reached by each backend")

    totals = 0
    done = 0
    for title, items, crate in [
        ("WebXR  (wxr-webxr)", webxr_items(), "wxr-webxr"),
        ("OpenXR (wxr-openxr)", openxr_items(not args.no_network), "wxr-openxr"),
        ("visionOS ARKit + CompositorServices (wxr-apple)", visionos_items(), "wxr-apple"),
    ]:
        if not items:
            print(f"\n## {title}: no spec found (fetch it, or point SDK at the headers)")
            continue
        h, t = report(title, items, source_of(crate))
        done += h
        totals += t

    if totals:
        print(f"\n**total: {done} / {totals}  ({100.0 * done / totals:.0f}%)**")


if __name__ == "__main__":
    main()
