#!/bin/bash

set -eo pipefail

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

function run_command() {
  "$@" > >(sed "s/^/$(_get_log_prefix)   /") | tee -a "$(git rev-parse --show-toplevel)/scripts/scripts.log" >> /dev/stderr
}
