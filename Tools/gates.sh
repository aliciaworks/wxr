#!/usr/bin/env bash
#
# The gates, locally - the loop.
#
# CI runs the same list on two hosts, and the two hosts are not redundant: the workspace-wide steps (test,
# `clippy --workspace`, `doc --workspace`) are the Ubuntu host's, because `wxr-openxr` is empty on Apple and
# the things it does have - examples, doc links - want crates that only exist off it. The visionOS half needs
# an Apple host. So this runs what its host can run and says so about the rest instead of pretending.
#
# Usage: Tools/gates.sh
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

step "fmt"
cargo fmt --all --check

if [[ "$(uname)" == "Darwin" ]]; then
  step "the host half - skipped: it is Ubuntu's (an empty wxr-openxr, and examples that want its crates)"
else
  step "test (host)"
  cargo test
  step "clippy (host)"
  cargo clippy --workspace --all-targets -- -D warnings

  if rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then
    step "clippy (wasm32)"
    cargo clippy --workspace --all-targets --target wasm32-unknown-unknown -- -D warnings
  else
    step "clippy (wasm32) - skipped, target not installed"
  fi

  step "doc (workspace)"
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
fi

if rustup target list --installed 2>/dev/null | grep -qx aarch64-apple-visionos; then
  step "clippy (visionOS)"
  cargo clippy -p wxr-apple --target aarch64-apple-visionos --all-targets -- -D warnings
  step "doc (visionOS)"
  RUSTDOCFLAGS="-D warnings" cargo doc -p wxr-apple --target aarch64-apple-visionos --no-deps
  step "build (visionOS) - CI's linked job, locally"
  cargo build -p wxr-apple --target aarch64-apple-visionos
else
  step "the Apple half - skipped, aarch64-apple-visionos not installed"
fi

printf '\n\033[1;32m== all gates this host has pass\033[0m\n'
