#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -eo pipefail

function usage() {
  echo "$0 <host>
  host: Either antworkerXYZ, hisbaanXY, or another name.
" >> /dev/stderr
  exit 1
}

set +u
if [[ -z "$1" ]]; then
  usage
fi

set -ux

host="$1"

repository_root="$(git rev-parse --show-toplevel)"
project_root="$repository_root/projects/ant-fs"

destination="$(anthost "$host")"

TARGET="$(get_rust_target "$destination")" make release -C "$project_root"

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
