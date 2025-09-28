#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euxo pipefail

project="$1"
env="$2"
host="$3"

repository_root="$(git rev-parse --show-toplevel)"

project_mode="$(get_service_mode "$project")"
script="install-$project_mode-service.sh"

version=$("$repository_root/scripts/$script" "$project" "$env" "$host")

"$repository_root/scripts/deploy-systemd.sh" "$project" "$host" "$version"
