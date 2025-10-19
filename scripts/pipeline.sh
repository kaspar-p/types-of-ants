#!/bin/bash

# Fully deploy every project, on every machine

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

set +x

function environments() {
  echo 'beta' # 'prod'
}

for env in $(environments); do
  for project in $(find_projects_in_env "$env"); do
    "$repository_root/scripts/deploy.sh" "$project" "$env"
  done 
done
