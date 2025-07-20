#!/bin/bash

#
# A script to build+install a rust binary. Does not affect the runtime of any running projects,
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
commit_datetime="$(git show -s --date=format:'%Y-%m-%d-%H-%M' --format=%cd "${commit_sha}")"
commit_number="$(git rev-list --count HEAD)"
install_datetime="$(date "+%Y-%m-%d-%H-%M")"
install_version="${commit_datetime}-${commit_sha}-v${commit_number}"

log "RESOLVING ENVIRONMENT [$project]..."

# Expose the environment ('beta', 'prod', ...) for other commands to pick up.
build_env="${repository_root}/secrets/${deploy_env}/build.env"
set -o allexport
# shellcheck disable=SC1090
source "$build_env"
set +o allexport

# Some projects require this for generating deterministic build hashes
export commit_sha

log "BUILDING [$project]..."

# Build the project
target=$(jq -r ".[\"$remote_host\"].[\"rust-target\"]" < "$repository_root/services.jsonc")
make -C "$project_src" -e TARGET="$target" release

log "INSTALLING [$project] ONTO [$remote_host]..."

install_dir="$remote_home/service/$project/$install_version"
run_command ssh2ant "$ant_worker_num" "
  mkdir -p $install_dir;
  mkdir -p $install_dir/secrets;
"

# Copy secrets into the install dir
secrets_dir="$repository_root/secrets/$deploy_env"
{
  cat "${build_env}"
} | ssh2ant "$ant_worker_num" "tee ${install_dir}/.env"
run_command rsync -Pa "${secrets_dir}/." "${remote_user}@${remote_host}:${install_dir}/secrets"

# Copy all other build/ files into the install dir
build_dir="$project_src/build"
build_mode="release"
run_command rsync -Pa "${build_dir}/${build_mode}/." "${remote_user}@${remote_host}:${install_dir}/"

# Interpret mustache template into the systemctl unit file
new_unit_path="$install_dir/$project.service"
INSTALL_DIR="$install_dir" HOME="$remote_home" VERSION="$install_version" mo "$project_src/$project.service.mo" | \
  ssh2ant "$ant_worker_num" "tee ${new_unit_path}"

# Write the installation manifest
ssh2ant "$ant_worker_num" "echo '{
  \"project\": \"$project\",
  \"project_type\": \"rust-binary\",
  \"version\": \"$install_version\",
  \"commit_sha\": \"$commit_sha\",
  \"commit_number\": \"$commit_number\"
  \"committed_at\": \"$commit_datetime\",
  \"installed_at\": \"$install_datetime\",
  \"unit_file\": \"$new_unit_path\"
}' > '${install_dir}/manifest.json'"

log "INSTALLED [$project] VERSION [$install_version]"
log "  when:        $(date -Iseconds)"
log "  install dir: $install_dir"
log "  version:     $install_version"
log "  unit file:   $new_unit_path"
