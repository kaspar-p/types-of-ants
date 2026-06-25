#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-archive-db/database-files"

reset=false
for arg in "$@"; do
  if [[ "$arg" == "--reset" ]]; then
    reset=true
  fi
done

if [[ "$reset" == true ]]; then
  echo "Wiping $PERSIST_DIR..."
  rm -rf "$PERSIST_DIR"
fi

# Write rendered compose to repo root so relative build contexts (./projects/...) resolve correctly.
cd "${repository_root}"
mo "projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > compose.dev.ant-archive-db.yaml

podman compose \
  --file compose.dev.ant-archive-db.yaml \
  up \
  --build \
  --force-recreate \
  ant-archive-db
