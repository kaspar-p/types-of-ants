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

function project_version() {
  local commit_sha
  commit_sha="$(git log --format='%h' -n 1)"
  
  local commit_datetime
  commit_datetime="$(git show -s --date=format:'%Y-%m-%d-%H-%M' --format=%cd "${commit_sha}")"
  
  local commit_number
  commit_number="$(git rev-list --count HEAD)"
  
  local install_version="${commit_number}-${commit_datetime}-${commit_sha}"

  echo "$install_version"
}

function get_services() {
  local repository_root
  repository_root="$(git rev-parse --show-toplevel)"

  cat "$repository_root/services.jsonc"
}

function get_docker_platform() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"][\"docker-platform\"]")"

  echo "$rust_target"
}

function get_docker_platform_arch() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"][\"docker-platform\"]" | cut -d '/' -f 2)"

  echo "$rust_target"
}

function get_rust_architecture() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"][\"rust-target\"]")"

  echo "$rust_target"
}

function smoke_test_systemd() {
  local project="$1"
  run_command sudo systemctl status "$project.service"
}
