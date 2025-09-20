#!/bin/bash

# Fully deploy every project, on every machine

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

set +x

function find_host_project_pairs_with_env() {
  local env="$1"
  cat "$repository_root/services.jsonc" | jq -rc "
    .hosts |
    to_entries | 
    map(
      select(.value.services | map(select(.env == \"$env\")) | length > 0) | 
      { host: .key, project: .value.services[] | .service }
    )[]"
}

function environments() {
  echo 'beta' # 'prod'
}

for env in $(environments); do
  for host_project_pair in $(find_host_project_pairs_with_env "$env"); do
    host=$(jq -r ".host" <<< "$host_project_pair")
    project=$(jq -r ".project" <<< "$host_project_pair")

    # if [[ $project == "ant-gateway" ]]; then continue; fi # Ignore because I don't know how to get new certs yet
    # if [[ $project == "ant-data-farm" ]]; then continue; fi # Ignore because prod has non-standard deployment.

    project_mode="$(get_service_mode "$project")"
    script="install-$project_mode-service.sh"

    version=$("$repository_root/scripts/$script" "$project" "$env" "$host")

    "$repository_root/scripts/deploy-systemd.sh" "$project" "$host" "$version"
  done 
done
