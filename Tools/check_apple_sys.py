#!/usr/bin/env python3
"""Check the hand-written Apple declarations in `wxr-apple`'s `sys.rs` against Apple's own headers.

`crates/wxr-apple/src/sys.rs` declares the C that the `objc2` crates do not: ARKit's C API, and the few
CompositorServices calls that generation skips. An `extern "C"` mistake compiles here and fails on a
device, and there is no device - so what this checks is the *name*, how many arguments it takes, and what
kind of thing it returns.

**The oracle is Apple's headers, reached without an Apple ID.** Xcode's headers are compressed with
`decmpfs` and an extractor that drops extended attributes produces files that read as empty, which is why
this workspace once believed they could not be read at all. They are on GitHub, in a mirror of the SDK:

    .../macos-sdk/main/MacOSX26.5.sdk/.../ARKit.framework/Versions/A/Headers
    .../macos-sdk/main/MacOSX26.5.sdk/.../CompositorServices.framework/Versions/A/Headers

Most of the visionOS C API is in there - it is `API_AVAILABLE(visionos(1.0), macos(26.0))`. What is *not*
is the handful Apple added for visionOS alone, and for those there are two fallbacks: WebKit's soft-link
headers, which restate a prototype for every symbol it links by hand, and mirrors of Apple's
documentation, which quote the prototype as `extern`.

The documentation is the last of the three on purpose. It also shows the *macro* Apple defines for
Objective-C callers - `#define ar_release(object) [(object) release]` - which reads like a function that
does not exist and is really a real function (`AR_EXTERN void ar_release(void *)`) that ObjC shadows. A
`#define` is therefore reported as a note and never as a failure: the header is what decides, and it says
there is a symbol.

    python3 Tools/check_apple_sys.py [crates/wxr-apple/src/sys.rs]

Network is required. If no source can be reached the check does not fail - it says so and exits 0, the way
the gates skip a toolchain this machine does not have.
"""

from __future__ import annotations

import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

SDK = (
    "https://raw.githubusercontent.com/alexey-lysiuk/macos-sdk/main/MacOSX26.5.sdk/System/Library/"
    "Frameworks/{}.framework/Versions/A/Headers/{}"
)
# The framework, and the headers of it that these declarations come from. A name in another header is
# reported as `no-oracle` rather than missed silently, so the list growing is a thing to notice.
HEADERS = {
    "ARKit": [
        "anchor", "data_provider", "hand_skeleton", "hand_tracking", "object", "session", "world_tracking",
    ],
    "CompositorServices": ["drawable", "view"],
}
WEBKIT = "https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebCore/PAL/pal/cocoa/{}.h"
WEBKIT_FILES = ["ARKitSoftLink", "CompositorServicesSoftLink"]
DOCS = [
    "https://raw.githubusercontent.com/zhangyu1818/apple-docs-for-rag/main/ARKit/md/arkit-{}.md",
    "https://raw.githubusercontent.com/mlshdev/llm-docs/main/apple-frameworks/pages/arkit/{}.md",
]
UA = "wxr-check-apple-sys (+https://github.com/aliciaworks/wxr)"

# The attributes and availability macros that sit between `AR_EXTERN` and the return type, and around it.
NOISE = re.compile(
    r"AR_OBJECT_RETURNS_RETAINED|AR_REFINED_FOR_SWIFT|AR_SWIFT_SENDABLE|AR_MT_UNSAFE|OS_REFINED_FOR_SWIFT"
    r"|__SWIFT_UNAVAILABLE_MSG\([^)]*\)|API_[A-Z_]+\([^)]*\)|_Nullable|_Nonnull|_Null_unspecified"
    r"|__attribute__\(\([^)]*\)\)"
)
SOFT_LINK = re.compile(
    r"SOFT_LINK_FUNCTION_FOR_HEADER\(\s*PAL,\s*(\w+),\s*(\w+),\s*([^,]+?),\s*\((.*?)\)\s*,\s*\(",
    re.S,
)
RUST_FN = re.compile(r"pub fn\s+(\w+)\s*\(([^;]*?)\)\s*(?:->\s*([^;]+?))?\s*;", re.S)

# Apple's C enums are spelled `*_t` exactly like its object handles are, so the name is what is left to
# tell them apart: a status or a correction is a small number, an anchor is a pointer.
ENUM_WORDS = (
    "status", "state", "result", "mode", "flag", "error", "quality", "fidelity", "correction", "chirality",
    "direction", "convention", "options",
)


def fetch(url: str) -> str | None:
    request = urllib.request.Request(url, headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.read().decode("utf-8", "replace")
    except (urllib.error.URLError, TimeoutError, OSError):
        return None


def arguments(text: str) -> list[str]:
    """The arguments of a C parameter list, with `void` meaning none."""
    text = text.strip()
    if not text or text == "void" or text == "()":
        return []
    return [part.strip() for part in text.split(",")]


def kind(name: str) -> str:
    """What sort of value a type is, coarsely enough that C and Rust can be compared at all."""
    name = " ".join(NOISE.sub(" ", name).split())
    name = name.rsplit(" ", 1)[-1] if name else name
    if not name or name == "void":
        return "void"
    if name == "bool":
        return "bool"
    if name.startswith("simd_") or "Float" in name:
        return "simd"
    if name.startswith("*") or name.startswith("Ar"):
        return "handle"
    if name.endswith("_t"):
        return "scalar" if any(word in name for word in ENUM_WORDS) else "handle"
    if name == "*":
        return "handle"
    return "scalar"


def from_headers() -> dict[str, tuple[list[str], str]]:
    """Every prototype the SDK mirror has.

    There are two ways of writing one, so two passes over the same text. ARKit puts `AR_EXTERN` in front
    of each and lets the arguments wrap; CompositorServices opens an `extern "C"` block once and then puts
    the return type on a line of its own.
    """
    raw = ""
    for framework, headers in HEADERS.items():
        for header in headers:
            page = fetch(SDK.format(framework, header + ".h"))
            if page is not None:
                raw += "\n" + page

    found = from_c(raw, r"(?:AR|CP)_EXTERN\s+(.+?)(\w+)\s*\(")
    for match in re.finditer(r"\n\s*([A-Za-z_][A-Za-z0-9_ ]*?)\s*\n\s*(\w+)\s*\(", raw):
        name, opening = match.group(2), match.end() - 1
        depth = 0
        for index in range(opening, len(raw)):
            if raw[index] == "(":
                depth += 1
            elif raw[index] == ")":
                depth -= 1
                if depth == 0:
                    found.setdefault(name, (arguments(raw[opening + 1 : index]), kind(match.group(1))))
                    break
    return found


def from_c(text: str, pattern: str) -> dict[str, tuple[list[str], str]]:
    """Prototypes out of C, by parenthesis matching rather than by regex - arguments contain commas, and
    attribute macros contain parentheses."""
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"//[^\n]*", " ", text)
    text = " ".join(text.split())

    found: dict[str, tuple[list[str], str]] = {}
    for match in re.finditer(pattern, text):
        name = match.group(2)
        depth, opening = 0, match.end() - 1
        for index in range(opening, len(text)):
            if text[index] == "(":
                depth += 1
            elif text[index] == ")":
                depth -= 1
                if depth == 0:
                    found[name] = (arguments(text[opening + 1 : index]), kind(match.group(1)))
                    break
    return found


def from_soft_links() -> dict[str, tuple[list[str], str]]:
    found: dict[str, tuple[list[str], str]] = {}
    for filename in WEBKIT_FILES:
        page = fetch(WEBKIT.format(filename))
        if page is None:
            continue
        for _, name, returns, parameters in SOFT_LINK.findall(page):
            found[name] = (arguments(parameters), kind(returns))
    return found


def from_documentation(name: str) -> tuple[list[str], str] | str | None:
    """What Apple's documentation says, from whichever mirror has it. `"macro"` is reported, not judged."""
    for mirror in DOCS:
        page = fetch(mirror.format(name))
        if not page:
            continue
        for line in page.splitlines():
            line = line.strip()
            if line.startswith("#define ") and f" {name}" in line:
                return "macro"
            match = re.match(rf"extern\s+(.+?)\s+{re.escape(name)}\s*\(([^)]*)\)", line)
            if match:
                return (arguments(match.group(2)), kind(match.group(1)))
    return None


def declared(path: Path) -> dict[str, tuple[list[str], str]]:
    """Each `pub fn` in `sys.rs`, as name -> (its arguments, the kind of thing it returns)."""
    found: dict[str, tuple[list[str], str]] = {}
    for match in RUST_FN.finditer(path.read_text()):
        name, arguments_text, returns = match.group(1), match.group(2), match.group(3)
        found[name] = (
            [part.strip() for part in arguments_text.split(",") if part.strip()],
            kind(returns or "void"),
        )
    return found


def main() -> int:
    source = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("crates/wxr-apple/src/sys.rs")
    if not source.exists():
        print(f"check-apple-sys: no {source}", file=sys.stderr)
        return 1

    rust = declared(source)
    if not rust:
        return 0

    headers = from_headers()
    if not headers:
        print("check-apple-sys: could not reach the SDK mirror; skipping", file=sys.stderr)
        return 0
    soft_links = from_soft_links()

    print(f"{len(rust)} declarations in {source.name}, against {len(headers)} prototypes in Xcode 26.5\n")
    problems = 0
    for name, (arguments_rust, returns_rust) in sorted(rust.items()):
        source_name, prototype = "the header", headers.get(name)
        if prototype is None:
            source_name, prototype = "WebKit", soft_links.get(name)
        if prototype is None:
            documented = from_documentation(name)
            if documented == "macro":
                # Not a failure: Apple defines this macro for Objective-C callers over a function the
                # header declares, so the symbol is there and the declaration is a function after all.
                print(f"note     {name}  (Apple documents a macro over it; the header decides)")
                continue
            if documented is None:
                print(f"no-oracle  {name}  (not in the headers, not in WebKit, no page declaring it)")
                continue
            prototype = documented
            source_name = "the documentation"

        arguments_apple, returns_apple = prototype
        if len(arguments_rust) != len(arguments_apple):
            print(
                f"ARITY    {name}\n"
                f"  here:  {len(arguments_rust)} {arguments_rust}\n"
                f"  {source_name}: {len(arguments_apple)} {arguments_apple}"
            )
            problems += 1
        elif returns_rust != returns_apple:
            print(
                f"RETURN   {name}\n"
                f"  here:  {returns_rust}\n"
                f"  {source_name}: {returns_apple}"
            )
            problems += 1
        else:
            print(f"ok       {name}  ({len(arguments_apple)} args -> {returns_apple}, from {source_name})")

    print(
        f"\n{problems} of {len(rust)} declarations disagree with Apple about a name, a count or a shape"
    )
    print(
        "the exact types are not compared - `*mut c_void` and `ar_session_t` are the same thing written "
        "twice - so what a mismatch does is print both shapes to be read"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
