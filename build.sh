#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"
SERVER_PACKAGE="mirrorproxy-server"
SERVER_BIN="mirrorproxy-server"
CLIENT_PACKAGE="mirrorproxy-cli"
CLIENT_BIN="mirrorproxy"
BUILD_STARTED=$SECONDS

case "${1:-}" in
  --force) export MIRRORPROXY_BUILD_FORCE=1; shift ;;
  -h|--help)
    echo 'Usage: ./build.sh [--force]'
    echo 'Build locked release artifacts; --force reinstalls dependencies and rebuilds the frontend.'
    exit 0 ;;
esac
if [[ $# -ne 0 ]]; then echo "unknown build option: $1" >&2; exit 2; fi
for command in node npm cargo rustc rustup; do
  command -v "$command" >/dev/null 2>&1 || { echo "missing build prerequisite: $command" >&2; exit 1; }
done
if [[ "$(node -p 'Number(process.versions.node.split(".")[0])')" -lt 24 ]]; then
  echo 'Node.js 24 or newer is required to build the Web console.' >&2; exit 1
fi
if [[ "$TARGET" == "x86_64-unknown-linux-musl" ]] && ! command -v musl-gcc >/dev/null 2>&1; then
  echo "missing musl-gcc; install musl-tools before building $TARGET" >&2; exit 1
fi
export MIRRORPROXY_BUILD_NPM_VERSION="$(npm --version)"
CACHE_HELPER="$ROOT_DIR/scripts/build-cache.mjs"
stage() {
  local label="$1" started=$SECONDS
  shift
  printf '[build] %s\n' "$label"
  "$@"
  printf '[build] %s completed in %ss\n' "$label" "$((SECONDS - started))"
}

cd "$ROOT_DIR/web"
if [[ "${MIRRORPROXY_BUILD_FORCE:-0}" != 1 ]] && node "$CACHE_HELPER" dependencies-valid; then
  echo '[build] Locked dependencies unchanged; reusing node_modules.'
else
  stage 'Install locked frontend dependencies' npm ci --prefer-offline --include=dev --no-fund
  node "$CACHE_HELPER" record-dependencies
fi
if [[ "${MIRRORPROXY_BUILD_FORCE:-0}" != 1 ]] && node "$CACHE_HELPER" web-valid; then
  echo '[build] Frontend inputs and output digests unchanged; reusing web/dist.'
  node scripts/check-browser-build.mjs
else
  stage 'Build frontend' npm run build
  node "$CACHE_HELPER" record-web
fi

cd "$ROOT_DIR"
stage 'Verify GeoIP inputs' bash scripts/fetch-geoip.sh
export GIT_COMMIT="${GIT_COMMIT:-$(git rev-parse HEAD)}"
if [[ -z "${BUILD_TIME:-}" ]]; then
  BUILD_TIME="$(node "$CACHE_HELPER" native-time "$TARGET")"
fi
export BUILD_TIME
if ! rustup target list --installed | grep -Fxq -- "$TARGET"; then
  stage 'Install Rust target' rustup target add "$TARGET"
fi
stage 'Build server and client' cargo build --locked --release --target "$TARGET" --package "$SERVER_PACKAGE" --package "$CLIENT_PACKAGE" --bins
node "$CACHE_HELPER" record-native "$TARGET"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
SERVER_BIN_PATH="$TARGET_DIR/$TARGET/release/$SERVER_BIN"
CLIENT_BIN_PATH="$TARGET_DIR/$TARGET/release/$CLIENT_BIN"
echo "Built server: $SERVER_BIN_PATH"
echo "Built client: $CLIENT_BIN_PATH"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$SERVER_BIN_PATH" "$CLIENT_BIN_PATH"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$SERVER_BIN_PATH" "$CLIENT_BIN_PATH"
else
  echo "warning: no SHA-256 command was found" >&2
fi
printf '[build] Total: %ss\n' "$((SECONDS - BUILD_STARTED))"
