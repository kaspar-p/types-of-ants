#!/bin/bash

set -euo pipefail

if (( $# < 1 )); then
  echo "$0: <deployment environment>
Saves a backup file from a remote database locally!
  
Arguments:
  <deployment environment>: either 'dev', 'beta', or 'prod'"
  exit 1
fi

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

backup_datetime="$(date "+%Y-%m-%d-%H-%M")"

backup_file="backups/${deploy_env}/${backup_datetime}-backup.sql"
mkdir -p "$(dirname "$backup_file")"

dbname="$(cat "$repository_root/secrets/$deploy_env/ant_data_farm_db.secret")"

output_file="backups/${deploy_env}/${backup_datetime}-${dbname}.bak.sql"

PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/ant_data_farm_password.secret")" pg_dump \
  --host "$ANT_DATA_FARM_HOST" \
  --port "$ANT_DATA_FARM_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/ant_data_farm_user.secret")" \
  --dbname "$dbname" \
  --create \
  --clean \
  --if-exists \
  --serializable-deferrable \
  --file "$output_file"

echo "$output_file"
