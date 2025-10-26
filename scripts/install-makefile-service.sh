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
host="$3"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi

if [[ -z "$1" ]] || [[ -z "$2" ]] || [[ -z "$3" ]]; then
  usage
fi
set -u

remote_user="ant"
remote_home="/home/$remote_user"
remote_host="$(anthost "$host")"
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
tmp_build_dir="$build_dir/$project.build.tmp"
run_command rm -rf "${tmp_build_dir}" # clear previous builds

run_command mkdir -p "${tmp_build_dir}"
run_command mkdir -p "${tmp_build_dir}/secrets"

build_mode="release"
run_command rm -rf "$build_dir/$build_mode/*"

make -C "$project_src" -e TARGET="$(get_rust_target "$remote_host")" release >> /dev/stderr

# Copy environment into the build directory.
log "... creating environment variables"
{
  cat "${build_cfg}"
  echo "PERSIST_DIR=$PERSIST_DIR"
} > "${tmp_build_dir}/.env"

# # Copy secrets into the build directory
# log "... copying secrets"
# local_secrets_dir="$repository_root/secrets/$deploy_env"
# for secret_name in $(jq -r '.secrets[]' < "$project_src/anthill.json"); do
#   log "... copying secret [$secret_name]"
#   cp "${local_secrets_dir}/${secret_name}.secret" "${tmp_build_dir}/secrets/${secret_name}.secret"
# done

# Copy all other build files into the build directory
cp -R "${build_dir}/${build_mode}/." "${tmp_build_dir}/"

# Interpret mustache template into the systemctl unit file
log "... creating unit file"
INSTALL_DIR="$INSTALL_DIR" HOME="$remote_home" VERSION="$install_version" mo "$project_src/$project.service.mo" > "${tmp_build_dir}/$project.service"

deployment_file_name="deployment.${project}.${install_version}.tar.gz"
log "... building deployment: ${deployment_file_name}"
tar -cz -C "${tmp_build_dir}" -f "${build_dir}/${deployment_file_name}" "."
rm -rf "${tmp_build_dir}"

deployment_size="$(du -hs "${build_dir}/${deployment_file_name}" | cut -f 1)"
log "... deployment file size: ${deployment_size}"

log "INSTALLING [$project] ONTO [$remote_host]..."
run_command ssh2ant "$host" "
  mkdir -p ${INSTALL_DIR};
  mkdir -p ${SECRETS_DIR};
"

# # Copy environment into the install dir
# {
#   cat "${build_cfg}"
#   echo "PERSIST_DIR=$PERSIST_DIR"
# } | ssh2ant "$host" "tee ${INSTALL_DIR}/.env" >> /dev/stderr

# # Copy secrets into the install dir
# local_secrets_dir="$repository_root/secrets/$deploy_env"
# for secret_name in $(jq -r '.secrets[]' < "$project_src/anthill.json"); do
#   log "... copying secret [$secret_name]"
#   run_command rsync -a "${local_secrets_dir}/${secret_name}.secret" "${remote_user}@${remote_host}:${SECRETS_DIR}/${secret_name}.secret"
# done

# # Copy all other build/ files into the install dir
# run_command rsync -a "${build_dir}/${build_mode}/." "${remote_user}@${remote_host}:${INSTALL_DIR}/"

# # Interpret mustache template into the systemctl unit file
# new_unit_path="$INSTALL_DIR/$project.service"
# INSTALL_DIR="$INSTALL_DIR" HOME="$remote_home" VERSION="$install_version" mo "$project_src/$project.service.mo" | \
#   ssh2ant "$host" "tee ${new_unit_path}" >> /dev/stderr

# # Write the installation manifest
# ssh2ant "$host" "echo '{
#   \"project\": \"$project\",
#   \"project_type\": \"makefile\",
#   \"version\": \"$install_version\",
#   \"commit_sha\": \"$commit_sha\",
#   \"commit_number\": \"$commit_number\",
#   \"committed_at\": \"$commit_datetime\",
#   \"installed_at\": \"$install_datetime\",
#   \"unit_file\": \"$new_unit_path\"
# }' > '${INSTALL_DIR}/manifest.json'"

# log "INSTALLED [$project] VERSION [$install_version]"
# log "  when:        $(date -Iseconds)"
# log "  install dir: $INSTALL_DIR"
# log "  version:     $install_version"
# log "  unit file:   $new_unit_path"

# Output the nondeterministic version
echo "$install_version"
