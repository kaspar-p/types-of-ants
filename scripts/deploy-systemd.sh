#!/bin/bash

#
# A script to deploy a rust binary. Expects that the rust binary has already been installed on the system.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  echo "USAGE: $0 <project-name> <version> <ant-worker-num>"
  exit 1
}

set +u
project="$1"
version="$2"
ant_worker_num="$3"
if [[ "$DEBUG" != "" ]]; then
  set -x
fi
set -u

if [[ -z "$project" ]] || [[ -z "$version" ]] || [[ -z "$ant_worker_num" ]]; then
  usage
fi

remote_home="/home/ant"

log "DEPLOYING [$project] VERSION [$version] ONTO [$ant_worker_num] ..."

deploy_datetime="$(date -Iminutes)"
install_dir="$remote_home/service/$project/$version"

# Cut over to the systemd service
new_unit_path="$install_dir/$project.service"

run_command ssh2ant "$ant_worker_num" "
  systemctl --user enable $new_unit_path;
  systemctl --user daemon-reload;
  systemctl --user restart '$project.service';
"

log "TRANSITIONED [$project] TO [$version]"
log "  when:        $deploy_datetime"
log "  install dir: $install_dir"
log "  version:     $version"
log "  unit file:   $new_unit_path"
