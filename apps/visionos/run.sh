#!/usr/bin/env bash
# Boot the visionOS Simulator, install the built wxr app, and (optionally) stream logs
# while it runs. The app is built by `build.sh` next to this script; run that first.
#
# Usage:
#   ./run.sh            # install and launch, returning immediately
#   ./run.sh --console  # install, launch, and stay attached to stdout/stderr
#   ./run.sh --log      # install and launch, then stream the unified log for the app
#
# Environment overrides:
#   WXR_SIM_UDID  the simulator device to use

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

# The verified simulator device. `simctl` takes a UDID and every call below uses this one.
UDID="${WXR_SIM_UDID:-13D6CCCE-7158-4247-AA00-756A28B9A8B7}"
BUNDLE_ID="com.aliciaworks.wxr"
APP_DIR="$SCRIPT_DIR/build/WxrApp.app"

if [[ ! -d "$APP_DIR" ]]; then
  echo "error: $APP_DIR does not exist; run build.sh first" >&2
  exit 1
fi

echo "==> booting the simulator ($UDID)"
# Already-booted is not an error here; `bootstatus` is what waits for it either way.
xcrun simctl boot "$UDID" 2>/dev/null || true
# The GUI Simulator app is optional - a booted device can be driven headlessly with
# `simctl`, and some Xcode installs do not ship the app. Bring it up if it is there.
SIMULATOR_APP="$DEVELOPER_DIR/Applications/Simulator.app"
if [[ -d "$SIMULATOR_APP" ]]; then
  open -a "$SIMULATOR_APP" || true
fi
xcrun simctl bootstatus "$UDID" -b

echo "==> installing $APP_DIR"
xcrun simctl install "$UDID" "$APP_DIR"

case "${1:-}" in
  --console)
    echo "==> launching $BUNDLE_ID (attached; Ctrl-C detaches)"
    exec xcrun simctl launch --console-pty "$UDID" "$BUNDLE_ID"
    ;;
  --log)
    echo "==> launching $BUNDLE_ID and streaming logs"
    xcrun simctl launch "$UDID" "$BUNDLE_ID"
    exec xcrun simctl spawn "$UDID" log stream --level debug \
      --predicate 'processImagePath CONTAINS "WxrApp"'
    ;;
  *)
    echo "==> launching $BUNDLE_ID"
    xcrun simctl launch "$UDID" "$BUNDLE_ID"
    ;;
esac
