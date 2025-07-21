#!/bin/bash

set -euo pipefail

function _get_log_prefix() {
  local dt
  dt="$(date -Iseconds)"
  local dir
  dir=$(basename "$(pwd)")
  echo "INFO [ $dt $USER@$(hostname) $dir ]"
}

function log() {
  echo "$(_get_log_prefix)" "$@" | tee -a "$(git rev-parse --show-toplevel)/scripts/scripts.log" >> /dev/stderr
}

function usage() {
  log "USAGE: $0 <project-name> <deploy-environment> <ant-worker-num>
          project-name: 'ant-gateway', 'ant-data-farm', ...
          deploy-environment: 'beta', 'prod', 'dev'
          ant-worker-num: 000, 001, ...
"
  exit 1
}

function run_command() {
  "$@" > >(sed "s/^/$(_get_log_prefix)   /") | tee -a "$(git rev-parse --show-toplevel)/scripts/scripts.log" >> /dev/stderr
}

function smoke_test_systemd() {
  local project="$1"
  run_command sudo systemctl status "$project.service"
}
