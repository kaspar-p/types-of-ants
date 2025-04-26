#!/bin/bash

#
# A script to deploy a rust binary. Expects that the rust binary has already been installed on the system.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  echo "USAGE: $0 <project-name> <version>"
  exit 1
}

set +u
project="$1"
version="$2"
if [[ "$DEBUG" != "" ]]; then
  set -x
fi
set -u

if [[ -z "$project" ]] || [[ -z "$version" ]]; then
  usage
fi

log "DEPLOYING [$project] version [$version]..."

deploy_datetime="$(date -Iminutes)"
install_dir="$HOME/service/$project/$version"

if [[ ! -d "$install_dir" ]]; then
  log "ERROR: could not find expected installation directory $install_dir"
  exit 1
fi

# Cut over to the systemd service
os="$(uname -s)"
SYSTEMD_DIR=""
if [[ "$os" = "Linux" ]]; then
  SYSTEMD_DIR="/etc/systemd/system/"
else
  log "ERROR: Cannot install systemd service on non-linux machine."
  exit 1
fi

new_unit_path="$install_dir/$project.service"
if [[ ! -f "$new_unit_path" ]]; then
  log "ERROR: unit file not found $new_unit_path"
  exit 1
fi

# Remove the current systemd service file
orig_unit_path="${SYSTEMD_DIR:?}/$project.service"
run_command sudo rm -rf "$orig_unit_path"

run_command sudo systemctl enable "$new_unit_path"

# Restart the service
run_command sudo systemctl daemon-reload
run_command sudo systemctl restart "$project.service"

log "TRANSITIONED [$project] TO [$version]"
log "  when:        $deploy_datetime"
log "  install dir: $install_dir"
log "  version:     $version"
log "  unit file:   $new_unit_path"
