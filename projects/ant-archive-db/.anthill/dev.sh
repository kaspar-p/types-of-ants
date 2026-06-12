#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-archive-db"

mo "${repository_root}/projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > "/tmp/ant-archive-db.compose.yaml"

docker-compose \
  --project-directory "${repository_root}" \
  --file /tmp/ant-archive-db.compose.yaml \
  up \
  --build \
  --force-recreate \
  ant-archive-db "${@:2}"
