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

if [[ "$project" != "ant-host-agent" ]]; then
  request="{ \"project\": \"$project\", \"version\": \"$version\" }"
  log "request: $(jq -c <<< "$request")"
  run_command curl \
    --no-progress-meter \
    -X POST \
    --fail-with-body \
    -w "\n" \
    -d "{ \"project\": \"$project\", \"version\": \"$version\" }" \
    -H 'Content-type: application/json' \
    "$remote_host:3232/service/service"
else
  log "... deploying old-school"

  remote_user="ant"
  remote_home="/home/$remote_user"
  repository_root="$(git rev-parse --show-toplevel)"

  deploy_datetime="$(date -Iminutes)"
  install_dir="$remote_home/service/$project/$version"

  # Cut over to the systemd service
  new_unit_path="$install_dir/$project.service"

  ssh "ant@$remote_host" "
    sudo -S systemctl disable '$project.service' <<< $(cat "$repository_root/secrets/ant_user.secret") || true;
    sudo -S systemctl enable '$new_unit_path'  <<< $(cat "$repository_root/secrets/ant_user.secret");
    sudo -S systemctl daemon-reload  <<< $(cat "$repository_root/secrets/ant_user.secret");
    sudo -S systemctl restart '$project.service'  <<< $(cat "$repository_root/secrets/ant_user.secret");
  "
fi

log "TRANSITIONED [$project] TO [$version]"
log "  when:        $deploy_datetime"
log "  version:     $version"
