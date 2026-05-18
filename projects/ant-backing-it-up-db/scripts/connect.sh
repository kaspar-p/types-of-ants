#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

# shellcheck disable=SC1091
source "$repository_root/scripts/lib.sh"

set +o allexport
# shellcheck disable=SC1090
source "$(deploy_env_filepath "$deploy_env")"
set -o allexport

secrets_dir=$(find_secrets_dir "$deploy_env")

PGPASSWORD="$(cat "$secrets_dir/ant_backing_it_up_db_password.secret")" psql \
  --host "$ANT_BACKING_IT_UP_DB_HOST" \
  --port "$ANT_BACKING_IT_UP_DB_PORT" \
  --username "$(cat "$secrets_dir/ant_backing_it_up_db_user.secret")" \
  --dbname "$(cat "$secrets_dir/ant_backing_it_up_db_db.secret")"
