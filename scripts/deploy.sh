#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

set +u
project="$1"
deploy_env="$2"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi

if [[ -z "$1" ]] || [[ -z "$2" ]]; then
  usage
fi
set -u

repository_root="$(git rev-parse --show-toplevel)"

project_mode="$(get_service_mode "$project")"
script="install-$project_mode-service.sh"

for host_project_pair in $(find_host_project_pairs_with_env "$deploy_env" | jq -rc "select(.project == \"$project\")"); do
  host=$(jq -r ".host" <<< "$host_project_pair")
  project=$(jq -r ".project" <<< "$host_project_pair")

  version=$("$repository_root/scripts/$script" "$project" "$deploy_env" "$host")
  "$repository_root/scripts/deploy-systemd.sh" "$project" "$host" "$version"
done
