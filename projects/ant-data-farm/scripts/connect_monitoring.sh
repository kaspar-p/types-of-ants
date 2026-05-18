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

secrets_dir="$(find_secrets_dir "$deploy_env")"

PGPASSWORD="$(cat "$secrets_dir/ant_db_monitoring_password.secret")" psql \
  --host "$ANT_DATA_FARM_HOST" \
  --port "$ANT_DATA_FARM_PORT" \
  --username monitoring \
  --dbname "$(cat "$secrets_dir/ant_data_farm_db.secret")"
