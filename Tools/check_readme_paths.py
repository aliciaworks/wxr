"""Every path and crate name the README's shell blocks name, checked to exist.

The README is the first thing anyone reads and the first thing anyone copies a command out of, and a command
that names a file or a crate that is not there wastes the reader's time before it teaches them anything. This
is not a check that those commands *work* - that is what `gates.sh` is for, by running them - it is only that
what they name is present. The two are different questions and the answers live in different places: a path
can be correct and the command still fail for want of a runtime, and a path can be wrong on a machine where
the command would otherwise have passed.

What is read is the ```sh blocks, line by line, after the trailing `# comment` is taken off - the comments in
those blocks are the prose about each command, and they are long enough to contain a `#` of their own. A line
that is a control structure (a `for`, a `while`, an `if`) is skipped rather than parsed: its body names
targets through a variable, and guessing at a variable is how a check starts reporting things that are not
true. The commands that survive are the ones with a path or a crate in them, and each of those is looked up.

Exit 1 when something named is missing, 0 when every name resolves.
"""

from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
README = os.path.join(ROOT, "README.md")

# `cargo test -p foo`, `cargo clippy -p foo`, `cargo build -p foo` - the crate name is the argument to `-p`.
CRATE_FLAG = re.compile(r"(?:^|\s)-p\s+(\S+)")
# `--example layers` - the example name is the argument to `--example`.
EXAMPLE_FLAG = re.compile(r"--example\s+(\S+)")
# A bare word after `python3` or `bash`, the two interpreters the README names. Everything else that starts a
# line in those blocks is `cargo`, which carries no path of its own.
INTERPRETER = re.compile(r"^(?:python3|bash)\s+(\S+)")
# A control-structure line, whose body is not a command this file can read.
CONTROL = re.compile(r"^\s*(?:for|while|if|case|done|else|elif|then|esac|\}|\\\s*$)")


def shell_blocks(lines: list[str]) -> list[str]:
    """Every line inside a ```sh block, and none of the fences or the prose between them."""
    out: list[str] = []
    inside = False
    for line in lines:
        stripped = line.strip()
        if stripped == "```sh":
            inside = True
            continue
        if stripped == "```" and inside:
            inside = False
            continue
        if inside:
            out.append(line)
    return out


def command_of(line: str) -> str:
    """The line with its trailing comment removed, which is where the README's prose about it lives."""
    return line.split("#", 1)[0].strip()


def missing_in(command: str) -> list[str]:
    """Every path or crate this command names that is not in the repository, in the order named."""
    if CONTROL.match(command):
        return []
    problems: list[str] = []

    # A crate named by `-p` lives at `crates/<name>`, and the workspace's own members are the crates under it.
    for crate in CRATE_FLAG.findall(command):
        if not os.path.isdir(os.path.join(ROOT, "crates", crate)):
            problems.append(f"crate `crates/{crate}`")

    # An example named by `--example` is a file under that crate's `examples/`. The crate is known from the
    # same command, and a command with an example and no `-p` is one this file cannot resolve - which is not
    # a case the README has, and rather than guess at a default the name is left alone.
    crates = CRATE_FLAG.findall(command)
    for example in EXAMPLE_FLAG.findall(command):
        if not crates:
            continue
        path = os.path.join(ROOT, "crates", crates[0], "examples", f"{example}.rs")
        if not os.path.isfile(path):
            problems.append(f"example `crates/{crates[0]}/examples/{example}.rs`")

    # A path handed to `python3` or `bash`, which is every other path the README names.
    for path in INTERPRETER.findall(command):
        if path.startswith("-"):  # a flag, not a path
            continue
        if not os.path.exists(os.path.join(ROOT, path)):
            problems.append(f"`{path}`")

    return problems


def main() -> int:
    with open(README, encoding="utf-8") as fh:
        lines = fh.read().splitlines()

    problems: list[tuple[str, str]] = []
    for line in shell_blocks(lines):
        command = command_of(line)
        if not command:
            continue
        for problem in missing_in(command):
            problems.append((command, problem))

    if not problems:
        print("every path and crate the README's shell blocks name is present")
        return 0
    for command, problem in problems:
        print(f"  {problem}  named by: {command}")
    print(f"\n{len(problems)} name(s) the README gives that are not in this repository")
    return 1


if __name__ == "__main__":
    sys.exit(main())
