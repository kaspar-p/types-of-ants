#!/bin/bash

set -euo pipefail

project="$1"

repository_root="$(git rev-parse --show-toplevel)"

set -o allexport
# shellcheck disable=SC1091
source "$repository_root/secrets/dev/build.cfg"
set +o allexport

if [[ -f "$repository_root/projects/$project/anthill.json" ]]; then
  PORT="$(cat "$repository_root/projects/$project/anthill.json" | jq '.ports.primary')"
  export PORT
  
  PRIMARY_PORT="$PORT"
  export PRIMARY_PORT
fi

if [[ -f "$repository_root/projects/$project/.anthill/dev.sh" ]]; then
  "$repository_root/projects/$project/.anthill/dev.sh" "${@:2}"
else
  echo "Unknown project: $project"
  exit 1
fi