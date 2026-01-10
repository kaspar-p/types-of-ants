#!/bin/bash

#
# A script to build a deployment.tar file, as per the docs/design/deployment-manifest.md file specification.
# Builds both makefile-based or docker-based services.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  echo "USAGE: $0 <project-name> [arch]
    project-name: 'ant-gateway', 'ant-data-farm', ...
    arch: 'arm', 'x86', or 'raspbian'. Defaults to all.
" >> /dev/stderr
  exit 1
}

set +u
project="$1"
arch="$2"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi

if [[ -z "$1" ]]; then
  usage
fi

if [[ -z "$2" ]]; then
  IFS=" " read -r -a arches <<< "$(get_all_architectures)"
else
  arches=("$arch")
fi
set -u

for arch in "${arches[@]}"; do
  remote_user="ant"
  remote_home="/home/$remote_user"
  repository_root="$(git rev-parse --show-toplevel)"
  project_src="$repository_root/projects/$project"

  if [[ ! -e "$project_src" ]]; then
    log "Project [$project] does not exist!"
    exit 1
  fi

  commit_number="$(git rev-list --count HEAD)"
  commit_sha="$(git log --format='%h' -n 1)"
  version="$(project_version)"

  log "RESOLVING ENVIRONMENT [$project]..."

  # # Expose the environment ('beta', 'prod', ...) for other commands to pick up.
  # build_cfg="${repository_root}/secrets/${deploy_env}/build.cfg"
  # set -o allexport
  # # shellcheck disable=SC1090
  # source "$build_cfg"
  # set +o allexport

  # Some projects require this for generating deterministic build hashes
  export commit_sha

  PERSIST_DIR="${remote_home}/persist/$project"
  INSTALL_DIR="${remote_home}/service/$project/$version"
  SECRETS_DIR="${INSTALL_DIR}/secrets"

  log "BUILDING [$project] FOR [$arch]..."

  # Build the project
  build_dir="$project_src/build"
  tmp_build_dir="$build_dir/tmp.$project.build"
  run_command rm -rf "${tmp_build_dir}" # clear previous builds

  run_command mkdir -p "${tmp_build_dir}"

  build_mode="release"
  run_command rm -rf "$build_dir/$build_mode/*"

  make -C "$project_src" \
    -e RUST_TARGET="$(get_rust_target "$arch")" \
    -e PROMETHEUS_OS="$(get_prometheus_os "$arch")" \
    -e PROMETHEUS_ARCH="$(get_prometheus_arch "$arch")" \
    release >> /dev/stderr

  # # Copy environment into the build directory.
  # log "... creating environment variables"
  # {
  #   cat "${build_cfg}"
  #   echo "PERSIST_DIR=$PERSIST_DIR"
  # } > "${tmp_build_dir}/.env"

  is_docker=false
  if is_project_docker "$project"; then
    is_docker=true
    log "... creating docker image"
    VERSION="$version" \
      PERSIST_DIR="$PERSIST_DIR" \
      INSTALL_DIR="$INSTALL_DIR" \
      SECRETS_DIR="$SECRETS_DIR" \
      run_command docker-compose build "${project}"
    
    log "... exporting docker image"
    docker_image_file="docker-image.tar"
    docker_image_path="$tmp_build_dir/$docker_image_file"
    run_command docker image save "${project}:${version}" -o "$docker_image_path"

    log "... copying docker-compose config"
    VERSION="$version" \
      PERSIST_DIR="$PERSIST_DIR" \
      INSTALL_DIR="$INSTALL_DIR" \
      SECRETS_DIR="$SECRETS_DIR" \
      docker-compose config "${project}" > "$tmp_build_dir/docker-compose.yml"
  fi

  # Create a small manifest.json file into the build directory
  echo "{
    \"commit_number\": \"$commit_number\"
  }" > "$tmp_build_dir/manifest.json"

  # Copy all other build files into the build directory
  cp -R "${build_dir}/${build_mode}/." "${tmp_build_dir}/"

  # Copy the systemd service template into the build directory
  cp "${project_src}/${project}.service" "${tmp_build_dir}/"

  deployment_file_name="${project}.${arch}.${version}.tar.gz"
  log "... building deployment file: ${deployment_file_name}"

  registry_dir="${repository_root}/build/registry"
  mkdir -p "$registry_dir"

  deployment_file_path="${registry_dir}/${deployment_file_name}"
  tar --disable-copyfile -cz -C "${tmp_build_dir}" -f "${deployment_file_path}" "."
  rm -rf "${tmp_build_dir}"

  deployment_size="$(du -hs "${deployment_file_path}" | cut -f 1)"
  log "... deployment file size: ${deployment_size}"

  log "... registering artifact"
  register_artifact "$project" "$version" "$arch" "$deployment_file_path"

  echo "$version"

# Interpret mustache template into the systemctl unit file
# log "... creating unit file"
# INSTALL_DIR="$INSTALL_DIR" HOME="$remote_home" VERSION="$version" mo "$project_src/$project.service.mo" > "${tmp_build_dir}/$project.service"

# log "INSTALLING [$project] ONTO [$remote_host]..."
# remote_deployment_file_store="${remote_home}/persist/ant-host-agent/fs/archives"
# remote_deployment_file_path="$remote_deployment_file_store/$deployment_file_name"
# log "... copying to $remote_deployment_file_path"
# run_command ssh2ant "$host" "
#   sudo -S mkdir -p $remote_deployment_file_store <<< $(cat "$repository_root/secrets/ant_user.secret") && echo
# "
# run_command scp "${deployment_file_path}" "${remote_user}@${remote_host}:/tmp/${deployment_file_name}"
# run_command ssh2ant "$host" "sudo -S mv /tmp/${deployment_file_name} ${remote_deployment_file_path} <<< $(cat "$repository_root/secrets/ant_user.secret") && echo"

# secrets_list="$(get_project_secrets "$project")"
# request="{
#   \"project\": \"$project\",
#   \"version\": \"$version\",
#   \"is_docker\": $is_docker,
#   \"secrets\": $secrets_list
# }"
# log "request: $(jq -c <<< "$request")"
# run_command curl \
#   --no-progress-meter \
#   --fail-with-body \
#   -X POST \
#   -w "\n" \
#   -d "${request}" \
#   -H 'Content-type: application/json' \
#   "$remote_host:3232/service/service-installation"

# # Output the nondeterministic version

done