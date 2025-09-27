#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/ant_zoo_storage_password.secret")" psql \
  --host "$ANT_ZOO_STORAGE_HOST" \
  --port "$ANT_ZOO_STORAGE_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/ant_zoo_storage_user.secret")" \
  --dbname "$(cat "$repository_root/secrets/$deploy_env/ant_zoo_storage_db.secret")"
