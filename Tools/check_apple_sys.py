#!/usr/bin/env python3
"""Check the hand-written Apple declarations in `wxr-apple`'s `sys.rs` against a real SDK.

`crates/wxr-apple/src/sys.rs` declares C functions that `objc2-compositor-services` does not bind and the
whole of ARKit's C API. Those declarations compile on any machine and are only *wrong* at link time or on a
device - unless there is an SDK to read, which is what this needs.

    python3 Tools/check_apple_sys.py <XROS.sdk> [crates/wxr-apple/src/sys.rs]

**The SDK has to have been extracted with extended attributes.** Most of Xcode's headers ship `decmpfs`
compressed, with the payload in the file's extended attributes and nothing but `NULLcanary` and NULs where the
text should be; an extractor that drops xattrs - plain `cpio`, for instance - produces a tree that looks right
and reads as empty. This checks for that first, because a symbol it cannot find in an empty file is a symbol it
must not report as missing.

What it can say is what the *name* is and how many arguments it takes, which is where a hand-written
declaration goes wrong. What it cannot say is that the types agree: `*mut c_void` and `void *` are the same
thing written two ways, and telling them apart needs the compiler rather than a script - so both signatures are
printed for the ones that disagree, and the difference is read rather than guessed.
"""

import re
import subprocess
import sys
from pathlib import Path

HEADER_DIRS = [
    "System/Library/Frameworks/ARKit.framework/Headers",
    "System/Library/Frameworks/CompositorServices.framework/Headers",
    "usr/include",
]

DECMPFS = (b"NULLcanary", b"cmpf")


def declared_in_rust(path: Path) -> dict[str, tuple[list[str], str]]:
    """Each `pub fn`, as name -> (its parameters, its return type)."""
    text = path.read_text()
    found: dict[str, tuple[list[str], str]] = {}
    for match in re.finditer(r"pub fn\s+(\w+)\s*\(([^;]*?)\)\s*(?:->\s*([^;]*?))?;", text, re.S):
        arguments = " ".join(match.group(2).split())
        parameters = [] if not arguments else [part.strip() for part in arguments.split(",")]
        found[match.group(1)] = (parameters, " ".join((match.group(3) or "void").split()))
    return found


def uncompressed(sdk: Path) -> bool:
    """Whether the headers have their contents, which is a question about the extraction rather than the SDK."""
    for directory in HEADER_DIRS:
        root = sdk / directory
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.h"))[:40]:
            head = path.open("rb").read(16)
            if head.startswith(DECMPFS):
                return False
    return True


def prototype(sdk: Path, symbol: str) -> str | None:
    """The declaration of `symbol`, from whichever header has it, with its attribute macros left on."""
    for directory in HEADER_DIRS:
        root = sdk / directory
        if not root.is_dir():
            continue
        result = subprocess.run(
            ["grep", "-rn", "-w", "-m1", symbol, "--include=*.h", str(root)],
            capture_output=True,
            text=True,
        )
        if not result.stdout.strip():
            continue
        path, _, _ = result.stdout.split("\n", 1)[0].partition(":")
        body = Path(path).read_text(errors="replace")
        # Comments first, because a doc comment here *calls* the function it documents, and a call reads as a
        # declaration to anything looking for `name(`.
        body = re.sub(r"/\*.*?\*/", " ", body, flags=re.S)
        body = re.sub(r"//[^\n]*", " ", body)
        for match in re.finditer(rf"\b{symbol}\s*\(", body):
            # The parameter list is the parentheses after the name, matched rather than searched for to its
            # semicolon: an attribute before the return type is a parenthesis pair of its own
            # (`API_AVAILABLE(visionos(1.0))`), and counting it would count a parameter that is not there.
            depth = 0
            end_of_arguments = None
            for index in range(match.end() - 1, len(body)):
                if body[index] == "(":
                    depth += 1
                elif body[index] == ")":
                    depth -= 1
                    if depth == 0:
                        end_of_arguments = index
                        break
            if end_of_arguments is None:
                continue
            start = body.rfind("\n", 0, match.start()) + 1
            return " ".join(body[start : end_of_arguments + 1].split())
    return None


def parameters(prototype: str) -> list[str]:
    """The parameter list of a C prototype, with each parameter's type kept and its name dropped."""
    inside = prototype[prototype.find("(") + 1 : prototype.rfind(")")]
    if not inside.strip() or inside.strip() == "void":
        return []
    out = []
    for piece in inside.split(","):
        words = [w for w in piece.replace("*", " * ").split() if w not in ("const", "volatile", "restrict")]
        if len(words) > 1 and re.fullmatch(r"\w+", words[-1]) and not words[-1].endswith("_t"):
            words = words[:-1]
        out.append(" ".join(words))
    return out


def main() -> int:
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    sdk = Path(sys.argv[1])
    source = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(
        "crates/wxr-apple/src/sys.rs"
    )
    if not source.exists():
        source = Path("~/Developer/wxr/crates/wxr-apple/src/sys.rs").expanduser()
    rust = declared_in_rust(source)

    if not uncompressed(sdk):
        print(
            "this SDK's headers are still compressed: they were extracted without extended attributes, so\n"
            "every file reads as empty and no symbol can be found in one. Extract it with a tool that keeps\n"
            "xattrs - `xtool sdk build Xcode.xip <dir>` does - and run this again.",
            file=sys.stderr,
        )
        return 2

    print(f"{len(rust)} declarations in {source.name}\n")
    problems = 0
    for name, (declared_args, declared_return) in sorted(rust.items()):
        real = prototype(sdk, name)
        if real is None:
            print(f"MISSING  {name}\n  rust: {declared_args} -> {declared_return}")
            problems += 1
            continue
        real_args = parameters(real)
        if len(real_args) != len(declared_args):
            print(
                f"ARITY    {name}\n"
                f"  rust:   {len(declared_args)} {declared_args}\n"
                f"  header: {len(real_args)} {real_args}\n"
                f"  ({real})"
            )
            problems += 1
        else:
            print(f"ok       {name}({len(real_args)} args)")

    print(f"\n{problems} of {len(rust)} declarations disagree with the SDK about a name or a count")
    print("the types themselves are for a compiler to judge; the two signatures are printed above where they differ")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
