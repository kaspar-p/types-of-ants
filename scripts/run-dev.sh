#!/bin/bash

set -euo pipefail

project="$1"

repository_root="$(git rev-parse --show-toplevel)"

set -o allexport
# shellcheck disable=SC1091
source "$repository_root/secrets/dev/build.cfg"
set +o allexport

if [[ $project = "ant-data-farm" ]]; then
  export ANT_DATA_FARM_PORT=3236
  export SECRETS_DIR="$repository_root/secrets/dev"
  export VERSION=dev
  export PERSIST_DIR="$repository_root/projects/ant-data-farm/database-files"
  docker-compose up --build ant-data-farm
elif [[ $project = "ant-on-the-web:server" ]]; then
  export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
  cargo run -p ant-on-the-web
elif [[ $project = "ant-on-the-web:website" ]]; then
  cd projects/ant-on-the-web/website && npm ci && npm run dev
else
  echo "Unknown project: $project, options are 'ant-data-farm', 'ant-on-the-web:server', and 'ant-on-the-web:website'"
  exit 1
fi