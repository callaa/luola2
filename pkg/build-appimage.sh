#!/bin/bash

set -e

if [ ! -d /app ]; then
	echo "This script should be run inside the build container"
	exit 1
fi

if [ ! -d /app/data/levels/luola2 ]; then
	echo "Luola 2 level pack not in data folder!"
	exit 1
fi

VERSION=$(grep version /app/Cargo.toml | head -n 1 | cut -d \" -f 2)

# Build
# SDL is built from sources so we get the exact version specified in Cargo.toml
# (version from build container's repo is almost always out of date)
cargo b --release \
	--target-dir /target \
	--features static

cd /target

# AppImage runtime was pre-downloaded during image build
export LDAI_RUNTIME_FILE=/usr/local/bin/appimage-runtime

# Create AppDir and manually copy in data files
# Note: we use --appimage-extract-and-run because fuse
# is not (easily) available in rootless podman.
linuxdeploy --appimage-extract-and-run \
	--appdir AppDir \
	--executable release/luola2 \
	--desktop-file /app/pkg/luola2.desktop \
	--icon-file /app/pkg/luola2.png \

cp -r /app/data AppDir/usr/bin/
rm -rf AppDir/usr/bin/data/levels/demos

# Package AppImage
linuxdeploy --appimage-extract-and-run --appdir AppDir --output appimage

mv Luola_2-x86_64.AppImage /build/Luola2-$VERSION-x86_64.Appimage
rm -rf AppDir
