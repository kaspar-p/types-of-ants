#!/bin/bash

#
# A script to deploy a rust binary. Expects that the rust binary has already been installed on the system.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  echo "USAGE: $0 <project-name> <ant-worker-num> <version>"
  exit 1
}

set +u
project="$1"
ant_worker_num="$2"
version="$3"
if [[ "$DEBUG" != "" ]]; then
  set -x
fi
set -u

if [[ -z "$project" ]] || [[ -z "$version" ]] || [[ -z "$ant_worker_num" ]]; then
  usage
fi

remote_host="$(anthost "$ant_worker_num")"

log "DEPLOYING [$project] VERSION [$version] ONTO [$ant_worker_num] ..."

deploy_datetime="$(date -Iminutes)"

run_command curl \
  -X POST \
  --silent \
  -w "\n" \
  -d "{ \"project\": \"$project\", \"version\": \"$version\" }" \
  -H 'Content-type: application/json' \
  "$remote_host:3232/service/service"

log "TRANSITIONED [$project] TO [$version]"
log "  when:        $deploy_datetime"
log "  version:     $version"
