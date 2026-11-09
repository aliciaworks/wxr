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
#   WXR_SIM_UDID  the simulator device to use (default: the first visionOS one this machine has)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# As in `build.sh`: the selected Xcode, rather than a path this script picked out for itself.
export DEVELOPER_DIR="${DEVELOPER_DIR:-$(xcode-select -p)}"

BUNDLE_ID="com.aliciaworks.wxr"
APP_DIR="$SCRIPT_DIR/build/WxrApp.app"

# A device UDID is a property of one machine at one moment - Xcode rebuilds the simulator and with it every
# device, which is how the id this used to name came to belong to nothing that exists. So it is looked up
# here rather than written down: the first visionOS device `simctl` calls available, whatever it is called
# and whatever its id turned out to be. Naming an id is still possible, and is what the override is for.
UDID="${WXR_SIM_UDID:-$(xcrun simctl list devices available \
  | awk '/^-- visionOS/{inside = 1; next} /^--/{inside = 0} inside' \
  | sed -nE 's/^.*\(([0-9A-Fa-f-]{36})\).*$/\1/p' \
  | head -1)}"

if [[ -z "$UDID" ]]; then
  echo "error: this machine has no available visionOS simulator; make one in Xcode, or set WXR_SIM_UDID" >&2
  exit 1
fi

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
