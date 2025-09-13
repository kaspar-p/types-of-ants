#!/bin/bash

#
# A script to build+install a rust binary. Does not affect the runtime of any running projects,
# is completely safe to run.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

set +u
project="$1"
deploy_env="$2"
ant_worker_num="$3"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi

if [[ -z "$1" ]] || [[ -z "$2" ]] || [[ -z "$3" ]]; then
  usage
fi
set -u

remote_user="ant"
remote_home="/home/$remote_user"
remote_host="$(anthost "$ant_worker_num")"
repository_root="$(git rev-parse --show-toplevel)"
project_src="$repository_root/projects/$project"

commit_sha="$(git log --format='%h' -n 1)"
commit_datetime="$(git show -s --date=format:'%Y-%m-%d-%H-%M' --format=%cd "${commit_sha}")"
commit_number="$(git rev-list --count HEAD)"
install_version="$(project_version)"
install_datetime="$(date "+%Y-%m-%d-%H-%M")"

log "RESOLVING ENVIRONMENT [$project]..."

# Expose the environment ('beta', 'prod', ...) for other commands to pick up.
build_cfg="${repository_root}/secrets/${deploy_env}/build.cfg"
set -o allexport
# shellcheck disable=SC1090
source "$build_cfg"
set +o allexport

# Some projects require this for generating deterministic build hashes
export commit_sha

PERSIST_DIR="${remote_home}/persist/$project"
INSTALL_DIR="${remote_home}/service/$project/$install_version"
SECRETS_DIR="${INSTALL_DIR}/secrets"

log "BUILDING [$project]..."

# Build the project
build_dir="$project_src/build"
build_mode="release"
run_command rm -rf "$build_dir/$build_mode/*"

make -C "$project_src" -e TARGET="$(get_rust_target "$remote_host")" release

log "INSTALLING [$project] ONTO [$remote_host]..."

run_command ssh2ant "$ant_worker_num" "
  mkdir -p ${INSTALL_DIR};
  mkdir -p ${SECRETS_DIR};
"

# Copy secrets into the install dir
local_secrets_dir="$repository_root/secrets/$deploy_env"
{
  cat "${build_cfg}"
  echo "PERSIST_DIR=$PERSIST_DIR"
} | ssh2ant "$ant_worker_num" "tee ${INSTALL_DIR}/.env"
run_command rsync -a "${local_secrets_dir}/." "${remote_user}@${remote_host}:${SECRETS_DIR}"

# Copy all other build/ files into the install dir
run_command rsync -a "${build_dir}/${build_mode}/." "${remote_user}@${remote_host}:${INSTALL_DIR}/"

# Interpret mustache template into the systemctl unit file
new_unit_path="$INSTALL_DIR/$project.service"
INSTALL_DIR="$INSTALL_DIR" HOME="$remote_home" VERSION="$install_version" mo "$project_src/$project.service.mo" | \
  ssh2ant "$ant_worker_num" "tee ${new_unit_path}"

# Write the installation manifest
ssh2ant "$ant_worker_num" "echo '{
  \"project\": \"$project\",
  \"project_type\": \"rust-binary\",
  \"version\": \"$install_version\",
  \"commit_sha\": \"$commit_sha\",
  \"commit_number\": \"$commit_number\",
  \"committed_at\": \"$commit_datetime\",
  \"installed_at\": \"$install_datetime\",
  \"unit_file\": \"$new_unit_path\"
}' > '${INSTALL_DIR}/manifest.json'"

log "INSTALLED [$project] VERSION [$install_version]"
log "  when:        $(date -Iseconds)"
log "  install dir: $INSTALL_DIR"
log "  version:     $install_version"
log "  unit file:   $new_unit_path"
