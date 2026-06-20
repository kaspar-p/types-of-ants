#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -eo pipefail

function usage() {
  echo "$0 <docker-platform> <arch>
  docker-platform: String like 'linux/arm64' or 'linux/amd64'
  arch: One of 'x86_64', 'aarch64', or 'armv7'
" >> /dev/stderr
  exit 1
}

set +u
if [[ -z "$1" ]]; then
  usage
fi
if [[ -z "$2" ]]; then
  usage
fi

set -ux

docker_platform="$1"
arch="$2"

repository_root="$(git rev-parse --show-toplevel)"
project_root="$repository_root/projects/ant-archive-storage"

version_tag="typesofants/ant-archive-storage:$arch-$(project_version)"
latest_tag="typesofants/ant-archive-storage:$arch-latest"

# Build the project binary
ah build ant-archive-storage --arch "$arch" --no-deploy

# Build the container and copy the binary
docker build \
  --build-context "root=$repository_root" \
  --build-arg "TARGET=$docker_platform" \
  --build-arg "ARCH=$arch" \
  --tag "$version_tag" \
  --tag "$latest_tag" \
  "$project_root"

docker push "$version_tag" --platform "$docker_platform"
docker push "$latest_tag" --platform "$docker_platform"
