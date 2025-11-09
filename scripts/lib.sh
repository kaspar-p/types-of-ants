#!/bin/bash

set -euo pipefail

function _get_log_prefix() {
  local dt
  dt="$(date -Iseconds)"
  echo "INFO [ $dt ]"
}

function log() {
  echo "$(_get_log_prefix)" "$@" | tee -a "$(git rev-parse --show-toplevel)/scripts/scripts.log" >> /dev/stderr
}

function usage() {
  echo "USAGE: $0 <project-name> <deploy-environment> <host>
    project-name: 'ant-gateway', 'ant-data-farm', ...
    deploy-environment: 'beta', 'prod', 'dev'
    host: 000, 001, antworker002, ...
" >> /dev/stderr

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

function find_host_project_pairs_with_env() {
  local env="$1"

  local repository_root
  repository_root="$(git rev-parse --show-toplevel)"

  cat "$repository_root/services.jsonc" | jq -rc "
    .hosts |
    to_entries | 
    map(
      select(.value.services | map(select(.env == \"$env\")) | length > 0) | 
      { host: .key, project: .value.services[] | .service }
    )[]"
}

function find_projects_in_env() {
  local env="$1"

  local repository_root
  repository_root="$(git rev-parse --show-toplevel)"

  cat "$repository_root/services.jsonc" | jq -rc "
    .hosts |
    to_entries[] |
    .value.services[] |
    select(.env == \"$env\").service" | sort | uniq
}

function get_project_type() {
  local project="$1"

  local repository_root
  repository_root="$(git rev-parse --show-toplevel)"

  jq -r '.project_type' < "$repository_root/projects/$project/anthill.json"
}

function is_project_docker() {
  local project="$1"

  test "$(get_project_type "$project")" == "docker"
}

function get_docker_platform() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"].docker_platform")"

  echo "$rust_target"
}

function get_docker_platform_arch() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"].docker_platform" | cut -d '/' -f 2)"

  echo "$rust_target"
}

function get_rust_target() {
  local host="$1"
  local arch
  arch="$(get_services | jq -r ".hosts[\"$host\"].architecture")"
  local rust_target
  rust_target="$(get_services | jq -r ".architectures[\"$arch\"].rust_target")"

  echo "$rust_target"
}

function smoke_test_systemd() {
  local project="$1"
  run_command sudo systemctl status "$project.service"
}
