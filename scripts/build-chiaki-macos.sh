#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
CHIAKI="$ROOT/third-party/chiaki-ng"
BUILD_DIR=${MOUSEPLAY_CHIAKI_BUILD_DIR:-"$ROOT/target/chiaki-ng-arm64-shared"}
VENV="$ROOT/target/chiaki-venv"

if ! command -v cmake >/dev/null 2>&1; then
    echo "cmake is required" >&2
    exit 1
fi

if [ ! -x "$VENV/bin/python" ]; then
    python3 -m venv "$VENV"
    "$VENV/bin/python" -m pip install protobuf
fi

git -C "$CHIAKI" submodule update --init --recursive

cmake -S "$CHIAKI" -B "$BUILD_DIR" \
    -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=ON \
    -DCMAKE_OSX_ARCHITECTURES=arm64 \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=13.0 \
    -DCHIAKI_ENABLE_GUI=OFF \
    -DCHIAKI_ENABLE_CLI=OFF \
    -DCHIAKI_ENABLE_TESTS=OFF \
    -DCHIAKI_ENABLE_STEAMDECK_NATIVE=OFF \
    -DCHIAKI_ENABLE_SETSU=OFF \
    -DCHIAKI_ENABLE_SPEEX=OFF \
    -DCHIAKI_ENABLE_FFMPEG_DECODER=ON \
    -DCHIAKI_ENABLE_PI_DECODER=OFF \
    -DCHIAKI_LIB_ENABLE_OPUS=OFF \
    -DCHIAKI_USE_SYSTEM_CURL=OFF \
    -DCHIAKI_ENABLE_STEAM_SHORTCUT=OFF \
    -DPYTHON_EXECUTABLE="$VENV/bin/python"

cmake --build "$BUILD_DIR" --target chiaki-lib --parallel "$(sysctl -n hw.ncpu)"

test -f "$BUILD_DIR/lib/libchiaki.dylib"
printf '%s\n' "libchiaki built at $BUILD_DIR/lib/libchiaki.dylib"
