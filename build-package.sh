#!/bin/bash

IMAGE=luola2-build

podman build -f pkg/Containerfile -t $IMAGE .

mkdir -p build

	#--user=$UID:$(id -g) \
podman run -it --rm \
	--userns=keep-id \
	--device /dev/fuse \
	--mount=type=bind,src="$(pwd)",dst=/app,relabel=private,ro=true \
	--mount=type=bind,src="$(pwd)/build",dst=/build,relabel=private \
	$IMAGE
