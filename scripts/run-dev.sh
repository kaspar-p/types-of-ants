#!/bin/bash

set -euo pipefail

project="$1"

repository_root="$(git rev-parse --show-toplevel)"

set -o allexport
# shellcheck disable=SC1091
source "$repository_root/secrets/dev/build.cfg"
set +o allexport

if [[ -f "$repository_root/projects/$project/anthill.json" ]]; then
  PORT="$(jq '.ports.primary' "$repository_root/projects/$project/anthill.json")"
  export PORT

  PRIMARY_PORT="$PORT"
  export PRIMARY_PORT

  METRICS_PORT="$(jq '.ports.metrics | if type == "object" then .port else . end // empty' "$repository_root/projects/$project/anthill.json")"
  if [[ -n "$METRICS_PORT" ]]; then
    export METRICS_PORT
  fi
fi

if [[ -f "$repository_root/projects/$project/.anthill/dev.sh" ]]; then
  "$repository_root/projects/$project/.anthill/dev.sh" "${@:2}"
else
  echo "Unknown project: $project"
  exit 1
fi