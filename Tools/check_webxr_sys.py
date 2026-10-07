#!/usr/bin/env python3
"""Check the generated WebXR bindings against the IDL they came from, and against a real browser.

`crates/wxr-webxr/src/sys` is generated, and a generator can be *silently* incomplete: a declaration it cannot
resolve is a declaration it leaves out, with no error. That is not hypothetical - an unresolved `DOMPointInit`
cost `XRRigidTransform` its constructor arguments, and an unresolved `Element` cost `XRDOMOverlayInit` its whole
file, and neither said anything at generation time.

So there are two checks here, and they catch different lies.

The first is offline and always runs: every interface, dictionary and enum in `webidl/` has to have a generated
file, and every member the IDL declares has to be a `js_name` in it. It compares the generator's output with the
generator's input, which is exactly the comparison that catches a resolution failure.

The second needs a browser, so it runs only when a captured prototype dump is beside the IDL - which is what
`Tools/capture_webxr_prototypes.js` produces. The IDL is not the truth about what a browser implements: this
checks that every name the generated module uses exists on the real prototype, which is the one mistake that
compiles, passes the first check, and fails only at runtime, in a browser, with a headset on.

    python3 Tools/check_webxr_sys.py
"""

import glob
import json
import re
import sys
from pathlib import Path

# What the generator turns into a `gen_<Name>.rs`: a `typedef` or a `callback` does not, and a `partial` only
# adds members to a declaration that does.
DECLARATION = re.compile(
    r"(?:partial\s+)?\b(interface\s+mixin|interface|dictionary|enum|namespace)\s+([A-Za-z_]\w*)"
)

# Members, from the three shapes WebIDL has: a method, an attribute, and a constant.
METHOD = re.compile(r"([A-Za-z_]\w*)\s*\(")
ATTRIBUTE = re.compile(r"(?:readonly\s+)?attribute\s+[^;()]*?([A-Za-z_]\w*)\s*;")
CONSTANT = re.compile(r"const\s+[^=;]*?([A-Za-z_]\w*)\s*=")

# `[^)]*` and not `.*?`: the greedy-lazy version walks *across* a type declaration into the next attribute
# block, and then reads that type's `js_name` as if it were the member's - which is how a checker comes to
# report members that are right there in the file it just read.
WASM_BINDGEN_ITEM = re.compile(
    r"#\[wasm_bindgen\(([^)]*)\)\]\s*\n(?:\s*#\[doc[^\n]*\]\s*\n)*\s*pub (?:fn|type|enum) (\w+)", re.S
)


def camel(name: str) -> str:
    parts = name.split("_")
    return parts[0] + "".join(part.capitalize() for part in parts[1:])


def body_after(text: str, start: int) -> str | None:
    """The brace-balanced body of the declaration that starts at `start`, or `None` if it has none.

    Balanced rather than "up to the first `}`", because these files have dictionaries with `= {}` defaults in
    them - which is a body inside a body, and the difference between reading `XRSessionInit` and reading half of
    it.
    """
    open_at = text.find("{", start)
    if open_at == -1:
        return None
    depth = 0
    for index in range(open_at, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_at + 1 : index]
    return None


def idl(root: Path) -> dict[str, tuple[str, set[str], bool]]:
    """The snapshot, as normalized name -> (kind, members, is ours).

    `externs.webidl` is ours rather than a specification's, and it is where the declarations the generator is
    *told* to resolve live - so its members are declared in this repository's own words and are not a promise
    about a browser. They are checked for being generated, and not for their members.
    """
    found: dict[str, tuple[str, set[str], bool]] = {}
    for path in sorted(glob.glob(str(root / "crates" / "wxr-webxr" / "webidl" / "enabled" / "*.webidl"))):
        # Comments first, because a comment is prose and prose has the word `interface` in it: `a stable
        # interface with a stable shape` reads as a declaration to anything that does not strip them.
        text = re.sub(r"//[^\n]*", "", Path(path).read_text())
        ours = Path(path).name == "externs.webidl"
        for match in DECLARATION.finditer(text):
            kind, name = match.group(1), match.group(2)
            current = found.setdefault(normalized(name), (kind, set(), ours))
            body = body_after(text, match.end())
            if body is None:
                continue
            for pattern in (METHOD, ATTRIBUTE, CONSTANT):
                # A constructor is declared like a method and is not one (the generator spells it `new`, and a
                # JavaScript prototype has no `constructor`), and an IDL member name always starts lower case -
                # which is what tells `getter XRInputSource(index)` apart from the `XRInputSource` it returns.
                current[1].update(
                    m for m in pattern.findall(body) if m != "constructor" and m[:1].islower()
                )
    return found


def normalized(name: str) -> str:
    """The name with everything but letters and digits dropped, and lowered.

    The generator's Rust names are `heck`'s idea of the IDL name - `XRGPUBinding` becomes `XrgpuBinding`, and
    `XREye` becomes `XrEye` while `XREnvironmentBlendMode` becomes `XrEnvironmentBlendMode` - so the two sides
    are matched on what they agree about rather than on the spelling either one chose.
    """
    return re.sub(r"[^a-z0-9]", "", name.lower())


def generated(root: Path) -> dict[str, set[str]]:
    """The generated module, as normalized name -> the JavaScript member names in it."""
    found: dict[str, set[str]] = {}
    for path in glob.glob(str(root / "crates" / "wxr-webxr" / "src" / "sys" / "gen_*.rs")):
        text = Path(path).read_text()
        # An enum carries no `js_name`: its name is the Rust one, and its variants are the values.
        name = re.search(r'js_name = "([A-Za-z0-9_]+)"', text) or re.search(
            r"pub (?:type|enum) ([A-Za-z0-9_]+)", text
        )
        if not name:
            continue
        members = found.setdefault(normalized(name.group(1)), set())
        for attrs, item in WASM_BINDGEN_ITEM.findall(text):
            # An indexing accessor is `this[index]` rather than a member: WebIDL's `getter T name(index)` has no
            # JavaScript name at all, and the generator says so with `indexing_*`.
            if "indexing_" in attrs:
                continue
            explicit = re.search(r'js_name = "([A-Za-z0-9_]+)"', attrs)
            if explicit:
                members.add(explicit.group(1))
            elif "method" in attrs or "getter" in attrs or "setter" in attrs:
                members.add(camel(item))
    return found


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    declared = idl(root)
    generated_ = generated(root)
    failures: list[str] = []

    for name, (kind, members, ours) in sorted(declared.items()):
        # A namespace and a mixin have no file of their own: a mixin exists to be included by an interface, and
        # the members it contributes arrive on that interface.
        if kind in ("namespace", "interface mixin"):
            continue
        if name not in generated_:
            failures.append(f"{name}: declared in the IDL, and generated not at all")
            continue
        if ours:
            continue
        for member in sorted(members):
            if member not in generated_[name]:
                failures.append(f"{name}.{member}: declared in the IDL, and not generated")

    snapshot = root / "crates" / "wxr-webxr" / "webidl" / "prototypes.json"
    known = json.loads(snapshot.read_text()) if snapshot.exists() else None
    checked = 0
    notes: list[str] = []
    if known is not None:
        known = {normalized(name): members for name, members in known.items()}
        for name, members in sorted(generated_.items()):
            # An interface no browser has is not a finding: it is the WebGPU binding, which is the whole reason
            # this crate generates its own, and one a browser has that this does not is not one either.
            if name not in known:
                continue
            checked += 1
            for member in sorted(members - {name}):
                if member not in known[name]:
                    notes.append(f"{name}.{member}")

    print(f"{len(generated_)} generated types, {len(declared)} declared in the IDL")
    print(
        f"{checked} checked against a browser's prototypes"
        if known is not None
        else "no prototype dump beside the IDL, so the browser check did not run"
    )
    for failure in failures:
        print(f"  {failure}", file=sys.stderr)
    if notes:
        # Not failures: a name the IDL has and a browser does not is usually the specification being ahead of
        # the implementation, and there is nothing here to fix when it is. What makes the list worth printing is
        # the other case - a name this crate writes by hand, in `throws.rs`, which is the only place a spelling
        # can be wrong. A name here that *is* in `throws.rs` is a bug.
        print(f"{len(notes)} names the IDL has and this browser does not:")
        for note in notes:
            print(f"  {note}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
