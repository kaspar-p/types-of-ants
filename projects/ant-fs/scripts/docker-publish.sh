#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euxo pipefail

repository_root="$(git rev-parse --show-toplevel)"
project_root="$repository_root/projects/ant-fs"

destination="$(anthost "$1")"

TARGET="$(get_rust_architecture "$destination")" make release -C "$project_root"

version_tag="typesofants/ant-fs:$(get_docker_platform_arch "$destination")-$(project_version)"
latest_tag="typesofants/ant-fs:$(get_docker_platform_arch "$destination")-latest"
destination_docker_platform="$(get_docker_platform "$destination")"

docker build \
  --build-context "root=$repository_root" \
  --build-arg "TARGET=$destination_docker_platform" \
  --tag "$version_tag" \
  --tag "$latest_tag" \
  "$project_root"

docker push "$version_tag" --platform "$destination_docker_platform"
docker push "$latest_tag" --platform "$destination_docker_platform"
