#!/usr/bin/env bash
#
# Take a `generate-visionos` artifact and make it the Apple backend's bindings.
#
# `wxr-apple` gets most of its ARKit declarations from a generated crate rather than by hand, and the crate is
# not published: it is a branch of this project's fork of `objc2`, whose ARKit is read from the visionOS SDK -
# the one SDK that has the C module the other backends' generated crate does not.
#
# Three things about that fork are easy to get wrong, and each of them has been got wrong here:
#
#   * a run rewrites its crate's `Cargo.toml` feature list *and* the imports its `lib.rs` opens with, so the
#     whole crate has to come across, not just its generated module;
#   * `src/generated` is a symlink into a top-level `generated/` directory upstream, so it has to be
#     dereferenced or the crate arrives with a `mod generated;` and no module;
#   * cargo resolves a whole workspace before it builds a member of it, so *every* manifest of the run goes in,
#     or the workspace stops resolving - which is a failure that looks like the crate being wrong.
#
# Usage: Tools/refresh_apple_generation.sh [<run-id>] [<fork-checkout>]
#
# The run id defaults to the newest successful `generate-visionos` run; the checkout defaults to
# `$HOME/.cache/apple-generation/objc2`, cloned if it is not there.
set -euo pipefail

REPO_FORK=https://github.com/aliciaworks/objc2.git
BRANCH=visionos-arkit
ARTIFACT=visionos-generated

run="${1:-}"
fork="${2:-$HOME/.cache/apple-generation/objc2}"

if [[ -z "${run}" ]]; then
  run="$(gh run list --repo aliciaworks/objc2 --workflow generate-visionos.yml \
    --status success --limit 1 --json databaseId --jq '.[0].databaseId')"
fi
echo "generation run: ${run}"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
gh run download "${run}" --repo aliciaworks/objc2 --name "${ARTIFACT}" --dir "${work}"

if [[ ! -d "${fork}/.git" ]]; then
  echo "cloning ${REPO_FORK} into ${fork}"
  mkdir -p "$(dirname "${fork}")"
  git clone --branch "${BRANCH}" "${REPO_FORK}" "${fork}"
fi
cd "${fork}"
git fetch origin "${BRANCH}" --quiet
git checkout "${BRANCH}" --quiet
git reset --hard "origin/${BRANCH}" --quiet

mkdir -p out
for crate in objc2-ar-kit objc2-compositor-services; do
  rm -rf "framework-crates/${crate}"
  cp -a "${work}/crates/${crate}" "framework-crates/"
done
for manifest in "${work}"/manifests/*.toml; do
  name="$(basename "${manifest}" .toml)"
  [[ -f "framework-crates/${name}/Cargo.toml" ]] && cp "${manifest}" "framework-crates/${name}/Cargo.toml"
done

git add -A framework-crates
if git diff --cached --quiet; then
  echo "nothing changed: the fork already holds this run's output"
else
  git -c user.name='Yukari Kaname' -c user.email='yukari@aliciaworks.com' commit --quiet -m \
    "$(cat <<'EOF'
the visionOS generation, applied whole

Every crate's manifest and the two crates this backend uses, from one run: the manifests because cargo
resolves a whole workspace before it builds a member of it, and those two crates whole because a run rewrites
more of them than their generated module.
EOF
)"
  git push origin "${BRANCH}"
fi

commit="$(git rev-parse HEAD)"
echo "fork at ${commit}"

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "${commit}" <<'PY'
import sys
from pathlib import Path

commit = sys.argv[1]
path = Path("crates/wxr-apple/Cargo.toml")
text = path.read_text()
import re
new, count = re.subn(r'(objc2-ar-kit = \{ git = "https://github.com/aliciaworks/objc2", rev = ")[0-9a-f]+',
                     rf"\g<1>{commit}", text)
if count == 0:
    anchor = 'objc2-compositor-services = "0.3"\n'
    assert anchor in text, "neither a pinned fork nor a place to add one"
    new = text.replace(anchor, anchor +
        "# ARKit is a visionOS module - its C API - and the generated crate for it does not have that module,\n"
        "# because upstream reads that framework from the iOS SDK. This fork reads it from the SDK that has it,\n"
        "# and is pinned by commit: a run rewrites the crate's feature list from the SDK it read.\n"
        f'objc2-ar-kit = {{ git = "https://github.com/aliciaworks/objc2", rev = "{commit}" }}\n', 1)
path.write_text(new)
print(f"wxr-apple now pins objc2-ar-kit at {commit}")
PY

echo
echo "check it with: cargo check -p wxr-apple --target aarch64-apple-visionos"
