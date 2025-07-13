#!/bin/bash

#
# A script to build+install a docker container. Does not affect the runtime of any running projects,
# is completely safe to run.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

function usage() {
  log "USAGE: $0 <project-name> <deploy-environment>
          project-name: 'ant-gateway', 'ant-data-farm', ...
          deploy-environment: 'beta', 'prod', 'dev'          
"
  exit 1
}

set +u
project="$1"
deploy_env="$2"
if [[ "$DEBUG" != "" ]]; then
 set -x
fi
set -u

if [[ -z "$1" ]] || [[ -z "$2" ]]; then
  usage
fi

repository_root="$(git rev-parse --show-toplevel)"
project_src="$repository_root/projects/$project"

commit_sha="$(git log --format='%h' -n 1)"
commit_datetime="$(git show -s --date=format:'%Y-%m-%d-%H-%M' --format=%cd "$commit_sha")"
commit_number="$(git rev-list --count HEAD)"
install_datetime="$(date "+%Y-%m-%d-%H-%M")"
install_version="$commit_datetime-$commit_sha-v$commit_number"

log "RESOLVING ENVIRONMENT [$project]..."

# Expose the environment ('beta', 'prod', ...) for other commands to pick up.
build_env="${repository_root}/secrets/${deploy_env}/build.env"
# shellcheck disable=SC1090
source "$build_env"
export VERSION="$install_version"

log "BUILDING [$project]..."

# Build the project
run_command make -C "$project_src" release
run_command docker-compose config "${project}"
run_command docker-compose build "${project}"

log "INSTALLING [$project]..."

install_dir="$HOME/service/$project/$install_version"
run_command mkdir -p "$install_dir"

# Copy secrets into the install dir
secrets_dir="$repository_root"
rm -f "${install_dir}/.env"
{
  cat "${build_env}"
  cat "${secrets_dir}/.env"
  echo "GIT_COMMIT_NUMBER=${commit_number}"
} >> "${install_dir}/.env"

# Copy all other build/ files into the install dir
build_dir="$project_src/build"
build_mode="release"
run_command cp -R "$build_dir/$build_mode/." "$install_dir"

# Interpret mustache template into the systemctl unit file
new_unit_path="$install_dir/$project.service"
HOME="$HOME" VERSION="$install_version" mo "$project_src/$project.service.mo" > "$new_unit_path"

# Write the installation manifest
echo "{
  \"project\": \"$project\",
  \"project_type\": \"docker-service\",
  \"version\": \"$install_version\",
  \"commit_sha\": \"$commit_sha\",
  \"commit_number\": \"$commit_number\",
  \"committed_at\": \"$commit_datetime\",
  \"installed_at\": \"$install_datetime\",
  \"unit_file\": \"$new_unit_path\"
}" > "$install_dir/manifest.json"

log "INSTALLED [$project] VERSION [$install_version]"
log "  when:        $(date -Iseconds)"
log "  install dir: $install_dir"
log "  version:     $install_version"
log "  unit file:   $new_unit_path"
