#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION="dev"
export PERSIST_DIR="$repository_root/projects/ant-zookeeper-db/database-files"

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

# Write the rendered file into the repo root so relative build contexts resolve correctly.
cd "${repository_root}"
mo "projects/ant-zookeeper/dev-fs/dev-fs/envs/docker-compose.yml" > compose.dev.yaml

podman compose \
  --file compose.dev.yaml \
  up \
  --build \
  --force-recreate \
  ant-zookeeper-db
