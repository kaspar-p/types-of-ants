#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  echo "USAGE: $0 <deploy-env>
  deploy-env: dev, beta, prod, ...
" >> /dev/stderr
}

set +u
if [[ -z "$1" ]]; then
  usage
fi

deploy_env="$1"

if [[ "$DEBUG" != "" ]]; then
 set -x
fi
set -u

repository_root="$(git rev-parse --show-toplevel)"

for host in $(find_hosts_with_agent_env "$deploy_env"); do
  remote_host="$(anthost "$host")"
  log "... COPYING TO [$remote_host]"

  local_secrets_dir="$repository_root/secrets/$deploy_env"
  for secret_path in "$local_secrets_dir"/*.secret; do
    secret_name="$(basename "$secret_path")"

    secret_value_base64="$(base64 -i "$secret_path")"
    request="{
      \"name\": \"$secret_name\",
      \"value\": \"$secret_value_base64\"
    }"
    log "... copying secret [$secret_name]"
    run_command curl \
      --no-progress-meter \
      -X POST \
      -w "\n" \
      -d "${request}" \
      -H 'Content-type: application/json' \
      "$remote_host:3232/secret/secret"
  done
done
