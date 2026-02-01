#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-data-farm/database-files"

mo "${repository_root}/projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > "/tmp/ant-data-farm.compose.yaml"

docker-compose \
  --project-directory "${repository_root}" \
  --file /tmp/ant-data-farm.compose.yaml \
  up \
  --build \
  --force-recreate \
  ant-data-farm "${@:2}"
