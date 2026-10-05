#!/bin/bash
# Launch smoke test: build for the simulator, launch the app, and fail if it
# doesn't survive.
#
# Why this exists: iOS 27 turned "app never adopted the UIScene lifecycle"
# from a logged warning into a hard launch crash (EXC_BREAKPOINT in
# UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption). Two builds
# shipped to TestFlight crashing on launch because *the archive built fine* --
# nothing in a compile step catches a launch-time trap.
#
# The runtime version matters more than anything else here. The crash only
# fires on iOS 27; the same binary launches fine on 26.x. A smoke test on an
# older runtime would have passed both broken builds, which is exactly the
# mistake this script exists to prevent. So: pick the NEWEST available iOS
# runtime, and say loudly which one was used.
set -euo pipefail

cd "$(dirname "$0")"

SCHEME="App"
BUNDLE_ID="com.foldpoker.app"
WORKSPACE="ios/App/App.xcworkspace"
ALIVE_AFTER_SECONDS="${ALIVE_AFTER_SECONDS:-5}"

echo "==> Picking the newest available iOS simulator runtime"
# simctl prints e.g. "iOS 27.0 (27.0 - 25A1234) - com.apple...iOS-27-0"
RUNTIME_LINE=$(xcrun simctl list runtimes 2>/dev/null \
  | grep -E '^iOS [0-9]' \
  | grep -v unavailable \
  | sort -t' ' -k2 -V \
  | tail -1)

if [ -z "$RUNTIME_LINE" ]; then
  echo "FAIL: no usable iOS simulator runtime installed."
  echo "      Install one with: xcodebuild -downloadPlatform iOS"
  exit 1
fi

RUNTIME_ID=$(echo "$RUNTIME_LINE" | sed -E 's/.* - (com\.apple\.CoreSimulator\.SimRuntime\.[^ ]+)$/\1/')
RUNTIME_VER=$(echo "$RUNTIME_LINE" | sed -E 's/^iOS ([0-9.]+) .*/\1/')
echo "    Using iOS $RUNTIME_VER ($RUNTIME_ID)"

# The crash this test was written for only reproduces on iOS 27+. Warn loudly
# rather than fail, so the test still runs usefully on an older machine --
# but nobody should mistake a pass here for real coverage.
MAJOR="${RUNTIME_VER%%.*}"
MIN_MAJOR="${SMOKE_MIN_IOS_MAJOR:-0}"
if [ "$MAJOR" -lt "$MIN_MAJOR" ]; then
  echo ""
  echo "FAIL: iOS $RUNTIME_VER is older than the required iOS $MIN_MAJOR."
  echo "      SMOKE_MIN_IOS_MAJOR=$MIN_MAJOR is set, so a pass on this"
  echo "      runtime would be meaningless -- the launch crashes we gate on"
  echo "      only reproduce on the current OS. Install it with:"
  echo "          xcodebuild -downloadPlatform iOS"
  exit 1
fi
if [ "$MAJOR" -lt 27 ]; then
  echo ""
  echo "    !! WARNING: iOS $RUNTIME_VER is older than iOS 27."
  echo "    !! The UIScene launch crash does NOT reproduce below iOS 27,"
  echo "    !! so a pass here does not prove the app launches on current"
  echo "    !! devices. Install the current runtime:"
  echo "    !!     xcodebuild -downloadPlatform iOS"
  echo ""
fi

echo "==> Creating a scratch simulator"
SIM_NAME="fold-smoke-$$"
SIM_ID=$(xcrun simctl create "$SIM_NAME" "iPhone 17 Pro" "$RUNTIME_ID" 2>/dev/null \
  || xcrun simctl create "$SIM_NAME" "iPhone 16 Pro" "$RUNTIME_ID")
echo "    $SIM_NAME ($SIM_ID)"

cleanup() {
  echo "==> Cleaning up simulator $SIM_ID"
  xcrun simctl shutdown "$SIM_ID" 2>/dev/null || true
  xcrun simctl delete "$SIM_ID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Booting"
xcrun simctl boot "$SIM_ID"
xcrun simctl bootstatus "$SIM_ID" -b

echo "==> Building for the simulator"
BUILD_DIR=$(mktemp -d)
xcodebuild \
  -workspace "$WORKSPACE" \
  -scheme "$SCHEME" \
  -configuration Debug \
  -sdk iphonesimulator \
  -destination "id=$SIM_ID" \
  -derivedDataPath "$BUILD_DIR" \
  CODE_SIGNING_ALLOWED=NO \
  build > "$BUILD_DIR/build.log" 2>&1 \
  || { echo "FAIL: build failed"; tail -40 "$BUILD_DIR/build.log"; exit 1; }

APP_PATH="$BUILD_DIR/Build/Products/Debug-iphonesimulator/$SCHEME.app"
[ -d "$APP_PATH" ] || { echo "FAIL: no .app at $APP_PATH"; exit 1; }

echo "==> Installing and launching"
xcrun simctl install "$SIM_ID" "$APP_PATH"
PID=$(xcrun simctl launch "$SIM_ID" "$BUNDLE_ID" | awk -F': ' '{print $2}')
echo "    launched pid $PID"

echo "==> Checking it is still alive after ${ALIVE_AFTER_SECONDS}s"
sleep "$ALIVE_AFTER_SECONDS"

if ! xcrun simctl spawn "$SIM_ID" launchctl list 2>/dev/null | grep -q "$BUNDLE_ID"; then
  # Fall back to checking the host process table; simctl's view can lag.
  if ! ps -p "$PID" > /dev/null 2>&1; then
    echo ""
    echo "FAIL: the app is not running ${ALIVE_AFTER_SECONDS}s after launch -- it crashed."
    echo "Most recent crash log:"
    # A simulator crash is written under the *device's* diagnostics directory,
    # not the host's ~/Library/Logs/DiagnosticReports -- check both, newest
    # first, or this prints nothing exactly when it is needed most.
    CRASH=$(ls -t \
      ~/Library/Logs/CoreSimulator/"$SIM_ID"/DiagnosticReports/App*.ips \
      ~/Library/Developer/CoreSimulator/Devices/"$SIM_ID"/data/Library/Logs/DiagnosticReports/App*.ips \
      ~/Library/Logs/DiagnosticReports/App-*.ips \
      2>/dev/null | head -1)
    if [ -n "$CRASH" ]; then
      echo "    ($CRASH)"
      head -40 "$CRASH"
    else
      echo "    (none found; check Console.app or re-run with ALIVE_AFTER_SECONDS larger)"
    fi
    exit 1
  fi
fi

echo ""
echo "PASS: app launched and survived ${ALIVE_AFTER_SECONDS}s on iOS $RUNTIME_VER"
