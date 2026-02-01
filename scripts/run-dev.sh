#!/bin/bash

set -euo pipefail

project="$1"

repository_root="$(git rev-parse --show-toplevel)"

set -o allexport
# shellcheck disable=SC1091
source "$repository_root/secrets/dev/build.cfg"
set +o allexport

if [[ -f "$repository_root/projects/$project/.anthill/dev.sh" ]]; then
  "$repository_root/projects/$project/.anthill/dev.sh"
elif [[ $project = "ant-data-farm" ]]; then
  export SECRETS_DIR="$repository_root/secrets/dev"
  export VERSION=dev
  export PERSIST_DIR="$repository_root/projects/ant-data-farm/database-files"
  docker-compose up --build ant-data-farm "${@:2}"
elif [[ $project = "ant-on-the-web" ]]; then
  export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
  cargo run -p ant-on-the-web
elif [[ $project = "ant-fs" ]]; then
  export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
  cd projects/ant-fs && cargo run
elif [[ $project = "ant-zookeeper" ]]; then
  export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
  export PERSIST_DIR="$repository_root/projects/ant-zookeeper/dev-fs"
  cd projects/ant-zookeeper && cargo run
else
  echo "Unknown project: $project"
  exit 1
fi