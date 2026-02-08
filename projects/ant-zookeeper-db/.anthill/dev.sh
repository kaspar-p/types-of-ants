#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-zookeeper-db/database-files"

mo "${repository_root}/projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > "/tmp/compose.yaml"

docker-compose \
  --project-directory "${repository_root}" \
  --file /tmp/compose.yaml \
  up \
  --build \
  --force-recreate \
  ant-zookeeper-db "${@:2}"
