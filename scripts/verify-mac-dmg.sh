#!/usr/bin/env bash
# Checks that a built CeeVee .dmg will open on a Mac, the way a downloaded copy does.
# Usage: scripts/verify-mac-dmg.sh path/to/CeeVee.dmg [--allow-adhoc-only]
# Fails if the app inside has a missing/broken code signature (which makes macOS say
# "CeeVee is damaged and can't be opened. You should move it to the Trash") or if the
# app exits right after launch.
set -euo pipefail

dmg="${1:?usage: verify-mac-dmg.sh <dmg>}"
work="$(mktemp -d)"
mnt="$work/mnt"
mkdir -p "$mnt"
cleanup() {
  pkill -f "$work/CeeVee.app" 2>/dev/null || true
  hdiutil detach "$mnt" -quiet 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

echo "== mount $dmg"
hdiutil attach "$dmg" -mountpoint "$mnt" -nobrowse -readonly -quiet
app_in_dmg="$mnt/CeeVee.app"
[ -d "$app_in_dmg" ] || { echo "FAIL: CeeVee.app not found in dmg"; ls "$mnt"; exit 1; }
[ -e "$mnt/Applications" ] || echo "note: no Applications shortcut in dmg"

echo "== copy out and mark as downloaded (quarantine), like a browser download"
ditto "$app_in_dmg" "$work/CeeVee.app"
xattr -w com.apple.quarantine "0083;$(printf '%x' "$(date +%s)");Safari;" "$work/CeeVee.app"
app="$work/CeeVee.app"

echo "== signature"
codesign -dvvv "$app" 2>&1 | sed -n '1,25p'
codesign --verify --deep --strict --verbose=2 "$app"
echo "signature valid"
# A sealed bundle has a CodeResources file; its absence is exactly what "damaged" means on arm64.
[ -f "$app/Contents/_CodeSignature/CodeResources" ] || { echo "FAIL: bundle is not sealed (no CodeResources)"; exit 1; }
codesign -dvv "$app" 2>&1 | grep -q 'Info.plist=not bound' && { echo "FAIL: Info.plist not bound to signature"; exit 1; }

echo "== architecture"
exe="$app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"
lipo -archs "$exe"
file "$exe"

echo "== launch smoke test"
"$exe" >"$work/stdout.log" 2>"$work/stderr.log" &
pid=$!
sleep "${SMOKE_SECONDS:-12}"
if ! kill -0 "$pid" 2>/dev/null; then
  echo "FAIL: app exited within ${SMOKE_SECONDS:-12}s"
  wait "$pid" || echo "exit code $?"
  echo "--- stdout"; cat "$work/stdout.log"; echo "--- stderr"; cat "$work/stderr.log"
  exit 1
fi
echo "app still running after ${SMOKE_SECONDS:-12}s (pid $pid)"
kill "$pid" 2>/dev/null || true
echo "OK"
