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
cargo b --release \
	--target-dir /target \
	--target x86_64-pc-windows-gnu \
	--features static

cd /target
mkdir luola2
cp x86_64-pc-windows-gnu/release/luola2.exe luola2/
cp /app/{README,LICENSE}.md luola2/
cp -r /app/data luola2/
zip -r -n .png:.jpeg luola2-$VERSION.zip luola2/ -x "data/levels/demos/*"
mv luola2-$VERSION.zip /build/
rm -rf luola2/
