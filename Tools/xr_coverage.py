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


def generated_aliases() -> dict:
    """C name -> the `Type::method` the generated crate names it by.

    A backend on the generated bindings calls `ar_session_t::new`, not `ar_session_create`, so the C name
    stops appearing the moment a backend switches over. The generated crate still carries it, as the
    `#[doc(alias = ...)]` on the method - and the method is qualified by its type on purpose: a bare `new` or
    `identifier` matches half a backend by accident, `ar_plane_anchor_t::identifier` matches the one call.
    """
    import glob

    out = {}
    pattern = str(
        Path.home()
        / ".cargo/git/checkouts/objc2-*/*/framework-crates/objc2-ar-kit/src/generated"
    )
    for d in glob.glob(pattern):
        for path in Path(d).glob("*.rs"):
            lines = path.read_text(errors="ignore").splitlines()
            current = None
            for i, line in enumerate(lines):
                impl = re.match(r"impl\s+(\w+)\s*\{", line)
                if impl:
                    current = impl.group(1)
                alias = re.match(r'\s*#\[doc\(alias = "((?:ar|cp)_\w+)"\)\]', line)
                if alias and current:
                    for after in lines[i + 1 : i + 60]:
                        fn = re.search(r"\bfn\s+(\w+)\s*[\(<]", after)
                        if fn:
                            out[alias.group(1)] = f"{current}::{fn.group(1)}"
                            break
    return out


CORE_FEATURES = [
    "DEPTH", "PLANES", "HIT_TEST", "LIGHT_ESTIMATION", "HAND_TRACKING", "ANCHORS",
    "LAYER_QUAD", "LAYER_CYLINDER", "LAYER_EQUIRECT", "LAYER_CUBE",
]


def core_coverage() -> None:
    """Which of the core's ten capabilities each backend implements.

    The native counts above are about a backend's own API and answer "how much of that is wired". This is the
    other direction, and it is the one that says whether the *unification* is done: the core is one vocabulary,
    a backend fills in the `Features` bits it can, and a native item with no core counterpart is one no
    backend will ever count - which is why the native percentage can never reach 100% by design.
    """
    crates = ["wxr-webxr", "wxr-openxr", "wxr-apple"]
    sources = {c: source_of(c) for c in crates}
    done = {c: 0 for c in crates}

    print("\n## The core's capabilities, per backend")
    print("| Feature | " + " | ".join(crates) + " |")
    print("| --- |" + " --- |" * len(crates))
    for feature in CORE_FEATURES:
        cells = []
        for c in crates:
            hit = f"Features::{feature}" in sources[c]
            done[c] += hit
            cells.append("yes" if hit else "")
        print(f"| `{feature}` | " + " | ".join(cells) + " |")
    print(
        "| **total** | "
        + " | ".join(f"**{done[c]}/{len(CORE_FEATURES)}**" for c in crates)
        + " |"
    )


def core_methods() -> list:
    """The methods the core's `Session` trait declares - the WebXR vocabulary, as Rust.

    Only that trait: `session.rs` also holds `Backend`, whose `connect` is not something a session fills in,
    and counting it is counting a method nobody is meant to implement.
    """
    # The whole directory, not one file: the trait and the vocabulary beside it are the same module, and a
    # tool that named one file broke the first time that module was split - which is what it is for, so it
    # reads whatever is there.
    text = "\n".join(
        p.read_text(errors="ignore") for p in sorted((ROOT / "crates/wxr/src/session").rglob("*.rs"))
    )
    start = text.index("pub trait Session")
    depth, block = 0, []
    for ch in text[start:]:
        block.append(ch)
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                break
    return sorted(set(re.findall(r"^\s{4}fn (\w+)", "".join(block), re.M)))


def session_impl(crate: str) -> set:
    """The `Session` methods a backend actually defines.

    Only what is inside its `impl wxr::Session for X` block: a helper with the same name as a trait method -
    and there are several, `depth` and `planes` among them - is not the method, and counting it is how a
    backend that offers one capability reads as 94%.
    """
    directory = ROOT / "crates" / crate / "src" / "session"
    path = ROOT / "crates" / crate / "src" / "session.rs"
    if directory.is_dir():
        text = "\n".join(p.read_text(errors="ignore") for p in sorted(directory.rglob("*.rs")))
    elif path.exists():
        text = path.read_text()
    else:
        return set()
    m = re.search(r"impl\s+(?:\w+::)?Session\s+for\s+\w+\s*\{", text)
    if not m:
        return set()
    depth, block = 0, []
    for ch in text[m.start() :]:
        block.append(ch)
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                break
    return set(re.findall(r"\bfn\s+(\w+)", "".join(block)))


def core_to_native() -> None:
    """Of the core's `Session` surface, how much each backend actually defines."""
    methods = core_methods()
    print(f"\n## WebXR -> native: the core's {len(methods)} Session methods, per backend")
    for crate in ["wxr-webxr", "wxr-openxr", "wxr-apple"]:
        hit = session_impl(crate)
        covered = [m for m in methods if m in hit]
        print(
            f"- {crate}: **{100 * len(covered) / len(methods):.0f}%**"
            f" ({len(covered)}/{len(methods)})"
        )


def webxr_interfaces() -> set:
    """Every interface, dictionary and enum the WebXR IDL declares."""
    names = set()
    idl = ROOT / "crates/wxr-webxr" / "webidl" / "enabled"
    for path in sorted(idl.glob("*.webidl")):
        text = re.sub(r"//.*", "", path.read_text(errors="ignore"))
        names |= set(
            re.findall(
                r"^\s*(?:partial\s+)?(?:interface|dictionary|enum|typedef)\s+(?:mixin\s+)?(\w+)",
                text,
                re.M,
            )
        )
    return names


# The interfaces the core has under a word of its own, or folds into one of its own.
#
# A rename is not a gap: the concept is there and the specification's name for it is the specification's. A
# collapse is not one either, and it is the more interesting of the two - the specification has four event
# interfaces because a browser dispatches four kinds of event, and this core has one `Event` enum because a
# frame loop drains one queue. Four names for one thing is the browser's shape, not a meaning the core is
# missing; the same is true of an array where there is a slice, of the three bindings where there is one
# import seam, and of the three depth-information subclasses where there is one `DepthInfo` and the
# backend's own texture. What is left after both lists is the work.
RENAMES = {
    "XRRigidTransform": "Pose",
    "XRReferenceSpaceType": "SpaceKind",
    "XRJointSpace": "HandJoint",
    "XRGPUSubImage": "SubImage",
}
COLLAPSED = (
    "XRSessionEvent",
    "XRInputSourceEvent",
    "XRInputSourcesChangeEvent",
    "XRReferenceSpaceEvent",
    "XRVisibilityMaskChangeEvent",
    "XRInputSourceArray",
    "XRWebGLLayer",
    "XRCPUDepthInformation",
    "XRGPUDepthInformation",
    "XRWebGLDepthInformation",
)


def webxr_gap() -> None:
    """The WebXR interfaces the core has no word for.

    The list "implement all of WebXR" is actually against: a member of an interface here cannot be translated
    by any backend, because the core has nothing to translate it into. Name matching is crude - the core calls
    `XRReferenceSpace` a `ReferenceSpace` - so this is a shortlist to read, not a verdict, and the entry that
    is a rename is the one to strike off first.
    """
    text = "\n".join(
        p.read_text(errors="ignore") for p in (ROOT / "crates/wxr/src").rglob("*.rs")
    ).lower()
    names = {n for n in webxr_interfaces() if n.startswith("XR")}
    missing = sorted(
        n for n in names if n.lower() not in text and n.lower().removeprefix("xr") not in text
    )
    # A `*Init` dictionary and a `*Set` are how the spec passes constructor arguments and collections; the
    # core takes those as arguments and slices, so they are not missing words. What is left is either a rename
    # - `XRRigidTransform` is `Pose` - or a concept the core does not have.
    dictionaries = [n for n in missing if n.endswith("Init") or n.endswith("Set")]
    renames = [n for n in missing if n in RENAMES]
    collapsed = [n for n in missing if n in COLLAPSED]
    rest = [
        n
        for n in missing
        if n not in dictionaries and n not in RENAMES and n not in COLLAPSED
    ]
    print(f"\n## WebXR XR-prefixed interfaces with no core name: {len(missing)} of {len(names)}")
    print(f"\n### the real gaps ({len(rest)}) - a concept the core has no word for, or a rename to check")
    print(", ".join(f"`{n}`" for n in rest))
    print(f"\n### dictionaries and sets ({len(dictionaries)}) - how the spec passes arguments and collections")
    print(", ".join(f"`{n}`" for n in dictionaries))
    print(f"\n### the same concept under a word of this core's own ({len(renames)})")
    print(", ".join(f"`{n}` is `{RENAMES[n]}`" for n in renames))
    print(f"\n### one thing the specification names many times ({len(collapsed)})")
    print(", ".join(f"`{n}`" for n in collapsed))



# The `openxr` crate's names for the commands, which is a rule and not a table.
#
# The crate is generated from the same registry this file reads, and it names a command by stripping `xr` and
# the vendor suffix and snake-casing what is left - `xrAcquireSwapchainImage` is `acquire_swapchain_image`,
# `xrLocateHandJointsEXT` is `locate_hand_joints`. So the name can be derived instead of written down, and a
# rule is what the count should use: forty commands copied by hand is forty that go stale the next time the
# crate is bumped. What is left over is where the crate disagrees with its own generator - the frame calls,
# which are split across the waiter and the stream, and the few that drop a word - and that is small enough to
# read.
OPENXR_RENAMES = {
    # The frame calls, split across the waiter and the stream, so neither is named after the command.
    "xrWaitFrame": "wait",
    "xrBeginFrame": "begin",
    "xrEndFrame": "end",
    # The destroys: the crate hangs them off the type's `Drop`, so what reaches one is the value existing -
    # and the value existing is the create that made it.
    "xrDestroyInstance": "create_instance",
    "xrDestroySession": "create_session",
    "xrDestroySwapchain": "create_swapchain",
    "xrDestroyActionSet": "create_action_set",
    "xrDestroyHandTrackerEXT": "create_hand_tracker",
    # And the ones that drop a word of their own.
    "xrGetSystem": "system",
    "xrEnumerateInstanceExtensionProperties": "enumerate_extensions",
    "xrAcquireSwapchainImage": "acquire_image",
    "xrWaitSwapchainImage": "wait_image",
    "xrReleaseSwapchainImage": "release_image",
    "xrEnumerateSwapchainImages": "enumerate_images",
    "xrCreateActionSpace": "create_space",
    "xrLocateSpace": "locate",
    "xrAttachSessionActionSets": "attach_action_sets",
    "xrGetVulkanGraphicsRequirementsKHR": "requirements",
    "xrGetVulkanGraphicsDeviceKHR": "vulkan_graphics_device",
}


def openxr_calls(commands) -> dict:
    """Every command, and the crate method its own name derives - the aliases this count uses.

    The crate is generated from the same registry this file reads, and it names a command by stripping `xr`,
    stripping the vendor suffix, and snake-casing what is left: `xrAcquireSwapchainImage` is
    `acquire_swapchain_image`, `xrLocateHandJointsEXT` is `locate_hand_joints`. The suffix is a run of capitals
    at the end and has to go *before* the snake casing - `EXT` snake-cased is `e_x_t`, which is how a rule that
    looked right matched nothing. So the name is derived rather than written down, and forty commands copied
    by hand are forty that go stale the next time the crate is bumped; `OPENXR_RENAMES` is only where the crate
    disagrees with its own generator.
    """
    out = {}
    for command in commands:
        name = re.sub(r"[A-Z]{2,}$", "", command.removeprefix("xr"))
        out[command] = OPENXR_RENAMES.get(command, re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower())
    return out


def spellings(name: str):
    """One word, as the specification spells it and as Rust does.

    A WebXR enum value is kebab-case and the Rust variant of the same word is CamelCase: `alpha-blend` is
    `AlphaBlend` and `world-space` is `WorldSpace`. That is spelling and not meaning - the member is reached
    when the core names the word, whichever way either side capitalises it - and matching verbatim is what made
    this count say the core had no word for `additive` while `Blend::Additive` sat in it. The aliases are the
    other matter and stay hand-written: a different *word* is a claim about meaning, and a claim needs a reader.
    """
    yield name
    words = re.split(r"[-_]", name)
    if len(words) > 1:
        yield "".join(w[:1].upper() + w[1:] for w in words)
        yield "_".join(words)
    elif name[:1].islower():
        yield name[:1].upper() + name[1:]
    snake = re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()
    if snake != name:
        yield snake


def reached(name: str, source: str, aliases=None) -> bool:
    """Whether the source names this member, in any of the ways a word can be written."""
    if any(re.search(rf"\b{re.escape(s)}\b", source) for s in spellings(name)):
        return True
    return bool(aliases) and name in aliases and re.search(rf"\b{re.escape(aliases[name])}\b", source)


def report(title, total, source, aliases=None):
    hit = {n for n in total if reached(n, source, aliases)}
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

    # Each backend is reached through whatever names it has for its own spec: the browser's through the
    # generated aliases above, and OpenXR's through the safe wrapper's method names - a backend written
    # against a wrapper never spells the command it calls.
    aliases = {
        "wxr-webxr": generated_aliases(),
        "wxr-openxr": openxr_calls(openxr_items(not args.no_network)),
        "wxr-apple": generated_aliases(),
    }
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
        h, t = report(title, items, source_of(crate), aliases.get(crate))
        done += h
        totals += t

    if totals:
        print(f"\n**total: {done} / {totals}  ({100.0 * done / totals:.0f}%)**")

    h, t = webxr_to_core()
    core_items = set()
    for p in (ROOT / "crates/wxr/src").rglob("*.rs"):
        core_items |= set(
            re.findall(r"pub (?:struct|enum|trait|fn|const|type) ([A-Za-z_]\w*)", p.read_text(errors="ignore"))
        )
    methods = core_methods()
    print(
        f"\n## WebXR -> core: **{100 * h / t:.0f}%** ({h}/{t} IDL members named by the core;"
        f" the core is {len(core_items)} public items plus {len(methods)} Session methods)"
    )
    core_to_native()
    webxr_api_coverage()
    webxr_gap()


# --- The whole WebXR API, not one specification of it. ---
#
# `webxr.webidl` is the WebXR Device API; the dozen files beside it are the modules, and Layers alone is as
# big as the core - a member of `XRLayer` or `XRHand` is as much WebXR as a member of `XRSession`. These
# redefine the three functions above so every specification is in the denominator; a later definition wins.

# What the domain leaves out: the externs, which are ours and not WebXR's, and the Community Group drafts,
# whose shape can still move under a backend. They are fetched and bound - a draft is worth being able to
# call - and not counted, because a number that moves with a draft is a number that means nothing.
WEBXR_DRAFTS = {"externs.webidl", "webxr-raw-camera-access.webidl", "webxr-real-world-meshing.webidl",
    "webxr-gamepad-haptics.webidl", "webxr-face-tracking.webidl",
    "webxr-body-tracking.webidl", "webxr-image-tracking.webidl"}


def _balanced(text, i):
    depth = 0
    for j in range(i, len(text)):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1 : j]
    return text[i + 1 :]


def _idl_members(path):
    raw = path.read_text(errors="ignore")
    text = re.sub(r"/\*.*?\*/", "", re.sub(r"//.*", "", raw), flags=re.S)
    out = []
    for m in re.finditer(r"\b(interface|dictionary|enum|namespace|mixin)\s+(?:mixin\s+)?(\w+)[^{;]*\{", text):
        kind, name = m.group(1), m.group(2)
        body = _balanced(text, m.end() - 1)
        if kind == "enum":
            out += [name + "." + v for v in re.findall(r'"([^"]+)"', body)]
            continue
        for chunk in re.split(r";\s*", body):
            c = " ".join(chunk.split())
            if not c:
                continue
            if "attribute" in c:
                out.append(name + "." + re.findall(r"(\w+)$", c)[0])
            elif c.startswith("const"):
                out.append(name + "." + (re.findall(r"(\w+)\s*=", c) or ["?"])[0])
            elif "(" in c:
                ops = re.findall(r"(\w+)\s*\(", c)
                out.append(name + "." + (ops[-1] if ops else "?"))
            else:
                words = re.findall(r"\w+", c)
                if words:
                    out.append(name + "." + words[-1])
    return out


def webxr_members():
    out = []
    idl = ROOT / "crates/wxr-webxr" / "webidl" / "enabled"
    for path in sorted(idl.glob("*.webidl")):
        if path.name in WEBXR_DRAFTS:
            continue
        out += _idl_members(path)
    return out


def webxr_items():
    return {q.split(".", 1)[-1] for q in webxr_members()}


def webxr_to_core():
    members = webxr_members()
    src = "\n".join(p.read_text(errors="ignore") for p in (ROOT / "crates/wxr/src").rglob("*.rs"))
    hit = [m for m in members if reached(m.split(".", 1)[1], src, ALIASES)]
    return len(hit), len(members)


def webxr_api_coverage():
    members = webxr_members()
    sources = {
        "wxr (core)": "\n".join(p.read_text(errors="ignore") for p in (ROOT / "crates/wxr/src").rglob("*.rs")),
        **{c: source_of(c) for c in ["wxr-webxr", "wxr-openxr", "wxr-apple"]},
    }
    print("\n## Of WebXR's " + str(len(members)) + " members (every specification), reached by:")
    for label, src in sources.items():
        hit = [m for m in members if reached(m.split(".", 1)[1], src, ALIASES)]
        print("- " + label + ": **" + str(round(100 * len(hit) / len(members))) + "%** (" + str(len(hit)) + "/" + str(len(members)) + ")")


# --- WebXR's names and the core's, where they differ. ---
#
# This is the one hand-written thing in this file, and on purpose: it is the claim a reader checks - "the core
# calls `requestReferenceSpace` a `space`" - and without it every count here is a floor, because the core
# renames every concept it shares and shares no name with the specification. Each line says: this member, the
# core serves it under this name.

ALIASES = {
    "requestReferenceSpace": "fn space",
    "requestAnimationFrame": "fn begin",
    "updateRenderState": "fn set_depth_range",
    "depthNear": "fn set_depth_range",
    "depthFar": "fn set_depth_range",
    "renderState": "fn blend",
    "environmentBlendMode": "fn blend",
    "framerate": "fn set_foveation",
    "fixationPoint": "fn set_foveation",
    "inputSources": "fn inputs",
    "visibilityState": "fn visibility",
    "interactionMode": "fn interaction_mode",
    "depthUsage": "fn depth_sensing",
    "depthDataFormat": "fn depth_sensing",
    "depthType": "fn depth_sensing",
    "depthActive": "fn depth_sensing",
    "requestHitTestSource": "fn hit_test_source",
    "requestHitTestSourceForTransientInput": "fn hit_test_source",
    "getHitTestResults": "fn hits",
    "getHitTestResultsForTransientInput": "fn hits",
    "createAnchor": "fn anchor",
    "delete": "fn release_anchor",
    "detectedPlanes": "fn planes",
    "planeSpace": "fn planes",
    "getDepthInformation": "fn depth",
    "createQuadLayer": "fn layer",
    "createCylinderLayer": "fn layer",
    "createEquirectLayer": "fn layer",
    "createCubeLayer": "fn layer",
    "createProjectionLayer": "fn layer",
    "requestLightProbe": "fn light_probe",
    "getLightEstimate": "fn light",
    "requestJointPose": "fn hand",
    "getSubImage": "fn sub_image",
    "getViewSubImage": "fn sub_image",
    "colorTextureWidth": "color_size",
    "colorTextureHeight": "color_size",
    "depthStencilTextureWidth": "depth_size",
    "depthStencilTextureHeight": "depth_size",
    "textureArrayIndex": "array_index",
}


if __name__ == "__main__":
    main()
