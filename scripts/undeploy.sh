#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

project="$1"
host="$2"

remote_host="$(anthost "$host")"

request="{
  \"project\": \"$project\"
}"
run_command curl \
  --no-progress-meter \
  -X DELETE \
  -w "\n" \
  -d "${request}" \
  -H 'Content-type: application/json' \
  "$remote_host:3232/service/service"

