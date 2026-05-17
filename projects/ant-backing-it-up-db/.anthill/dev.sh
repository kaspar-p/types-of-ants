#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-backing-it-up-db/database-files"

TEMP_COMPOSE='/tmp/ant-backing-it-up-compose.yaml'
mo "${repository_root}/projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > "$TEMP_COMPOSE"

docker-compose \
  --project-directory "${repository_root}" \
  --file "$TEMP_COMPOSE" \
  up \
  --build \
  --force-recreate \
  ant-backing-it-up-db "${@:2}"
