#!/bin/bash

#
# A script to build+install a docker container. Does not affect the runtime of any running projects,
# is completely safe to run.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  log "USAGE: $0 <project-name> <deploy-environment> <ant-worker-num>
          project-name: 'ant-gateway', 'ant-data-farm', ...
          deploy-environment: 'beta', 'prod', 'dev'
          ant-worker-num: 000, 001, ...
"
  exit 1
}

set +u
project="$1"
deploy_env="$2"
ant_worker_num="$3"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi
set -u

if [[ -z "$1" ]] || [[ -z "$2" ]] || [[ -z "$3" ]]; then
  usage
fi

remote_user="ant"
remote_home="/home/$remote_user"
remote_host="$(anthost "$ant_worker_num")"
repository_root="$(git rev-parse --show-toplevel)"
project_src="$repository_root/projects/$project"

commit_sha="$(git log --format='%h' -n 1)"
commit_datetime="$(git show -s --date=format:'%Y-%m-%d-%H-%M' --format=%cd "$commit_sha")"
commit_number="$(git rev-list --count HEAD)"
install_datetime="$(date "+%Y-%m-%d-%H-%M")"
install_version="$commit_datetime-$commit_sha-v$commit_number"

log "RESOLVING ENVIRONMENT [$project]..."

# Expose the environment ('beta', 'prod', ...) for other commands to pick up.
build_cfg="${repository_root}/secrets/${deploy_env}/build.cfg"
set -o allexport
# shellcheck disable=SC1090
source "$build_cfg"
set +o allexport
export VERSION="$install_version"

install_dir="${remote_home}/service/$project/$install_version"
export INSTALL_DIR="$install_dir"
export PERSIST_DIR="${remote_home}/persist"

remote_secrets_dir="${install_dir}/secrets"

log "BUILDING [$project]..."

# Build the project remotely
run_command make -C "$project_src" release
DOCKER_HOST="ssh://${remote_user}@${remote_host}" run_command docker-compose build "${project}"

# Install the project files
log "INSTALLING [$project] ONTO [$remote_host]..."
run_command ssh2ant "$ant_worker_num" "
  mkdir -p ${install_dir};
  mkdir -p ${remote_secrets_dir}
"

# Copy dockerfile into install dir
docker-compose config "${project}" | \
  ssh2ant "$ant_worker_num" "tee ${install_dir}/docker-compose.yml"

# Copy environment into the install dir
secrets_dir="${repository_root}/secrets/${deploy_env}"
rm -f "${install_dir}/.env"
{
  cat "${build_env}"
} | ssh2ant "$ant_worker_num" "tee ${install_dir}/.env"

# Copy secrets into the install dir
run_command scp -r "${secrets_dir}/." "${remote_host}:${remote_secrets_dir}/"

# Copy all the docker image and build/ files into the install dir
build_dir="${project_src}/build"
build_mode="release"
run_command scp -r "${build_dir}/${build_mode}/." "${remote_host}:${install_dir}/"

# Interpret mustache template into the systemctl unit file
new_unit_path="${install_dir}/${project}.service"
INSTALL_DIR="$install_dir" HOME="$HOME" VERSION="$install_version" mo "$project_src/$project.service.mo" | \
  ssh2ant "$ant_worker_num" "tee $new_unit_path"

# Write the installation manifest
ssh2ant "$ant_worker_num" "echo '{
  \"project\": \"$project\",
  \"project_type\": \"docker-service\",
  \"version\": \"$install_version\",
  \"commit_sha\": \"$commit_sha\",
  \"commit_number\": \"$commit_number\",
  \"committed_at\": \"$commit_datetime\",
  \"installed_at\": \"$install_datetime\",
  \"unit_file\": \"$new_unit_path\"
}' > '${install_dir}/manifest.json'"

log "INSTALLED [$project] VERSION [$install_version] ONTO [$remote_host]"
log "  when:        $(date -Iseconds)"
log "  install dir: $install_dir"
log "  version:     $install_version"
log "  unit file:   $new_unit_path"
