#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

set -o allexport
# shellcheck disable=SC1091
source "$repository_root/secrets/dev/build.cfg"
set +o allexport

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION=dev
export PERSIST_DIR="$repository_root/projects/ant-monitor-fe/dev-persist"

compose_file="/tmp/ant-monitor-fe-compose.yaml"
mo "${repository_root}/projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > "$compose_file"

docker-compose \
  --project-directory "${repository_root}" \
  --file "$compose_file" \
  up \
  --build \
  --force-recreate \
  ant-monitor-fe "${@:2}"