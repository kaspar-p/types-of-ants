#!/bin/bash

#
# A script that exits successfully if the dockerized systemd service is running at first
# glance.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  log "USAGE: $0 <project-name>"
  exit 1
}

set +u
project="$1"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi
set -u

if [[ -z "$project" ]]; then
  usage
fi

log "ENSURING SYSTEMD SERVICE RUNNING..."
run_command sudo systemctl status "$project.service"
