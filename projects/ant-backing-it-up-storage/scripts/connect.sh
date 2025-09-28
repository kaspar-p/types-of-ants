#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/ant_backing_it_up_storage_password.secret")" psql \
  --host "$ANT_BACKING_IT_UP_STORAGE_HOST" \
  --port "$ANT_BACKING_IT_UP_STORAGE_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/ant_backing_it_up_storage_user.secret")" \
  --dbname "$(cat "$repository_root/secrets/$deploy_env/ant_backing_it_up_storage_db.secret")"
