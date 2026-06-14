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

consul_res="$(ssh2ant 004 "curl http://localhost:3245/v1/catalog/service/ant-archive-db")"
ip="$(echo "$consul_res" | jq -r '.[0].Address')"
port="$(echo "$consul_res" | jq -r '.[0].ServicePort')"

secrets_dir=$(find_secrets_dir "$deploy_env")

PGPASSWORD="$(cat "$secrets_dir/ant_archive_db_password.secret")" psql \
  --host "$ip" \
  --port "$port" \
  --username "$(cat "$secrets_dir/ant_archive_db_user.secret")" \
  --dbname "$(cat "$secrets_dir/ant_archive_db_db.secret")"
