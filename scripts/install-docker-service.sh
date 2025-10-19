#!/bin/bash

#
# A script to build+install a docker container. Does not affect the runtime of any running projects,
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
set -u

if [[ -z "$1" ]] || [[ -z "$2" ]] || [[ -z "$3" ]]; then
  usage
fi

deploy_build "$project" "$deploy_env" "$ant_worker_num"

DOCKER_HOST="ssh://${remote_user}@${remote_host}" run_command docker-compose build "${project}"

# Install the project files
log "INSTALLING [$project] ONTO [$remote_host]..."
run_command ssh2ant "$ant_worker_num" "
  mkdir -p ${INSTALL_DIR};
  mkdir -p ${SECRETS_DIR}
"

# Copy dockerfile into install dir
docker-compose config "${project}" | \
  ssh2ant "$ant_worker_num" "tee ${INSTALL_DIR}/docker-compose.yml" >> /dev/stderr

# Copy environment into the install dir
rm -f "${INSTALL_DIR}/.env"
{
  cat "${build_cfg}"
  echo "PERSIST_DIR=$PERSIST_DIR"
} | ssh2ant "$ant_worker_num" "tee ${INSTALL_DIR}/.env" >> /dev/stderr

# Copy secrets into the install dir
local_secrets_dir="${repository_root}/secrets/${deploy_env}"
for secret_name in $(jq -r '.secrets[]' < "$project_src/anthill.json"); do
  log "... copying secret [$secret_name]"
  run_command rsync -a "${local_secrets_dir}/${secret_name}.secret" "${remote_user}@${remote_host}:${SECRETS_DIR}/${secret_name}.secret"
done

# Copy all the docker image and build/ files into the install dir
build_dir="${project_src}/build"
build_mode="release"
run_command scp -r "${build_dir}/${build_mode}/." "${remote_host}:${INSTALL_DIR}/"

# Interpret mustache template into the systemctl unit file
new_unit_path="${INSTALL_DIR}/${project}.service"
INSTALL_DIR="$INSTALL_DIR" HOME="$HOME" VERSION="$install_version" mo "$project_src/$project.service.mo" | \
  ssh2ant "$ant_worker_num" "tee $new_unit_path" >> /dev/stderr

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
}' > '${INSTALL_DIR}/manifest.json'"

log "INSTALLED [$project] VERSION [$install_version] ONTO [$remote_host]"
log "  when:        $(date -Iseconds)"
log "  install dir: $INSTALL_DIR"
log "  version:     $install_version"
log "  unit file:   $new_unit_path"

# Output
echo "$install_version"
