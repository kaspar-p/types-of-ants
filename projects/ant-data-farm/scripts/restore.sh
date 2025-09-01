#!/bin/bash

set -euo pipefail

deploy_env="$1"
backup_filepath="$2"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

if [[ "$deploy_env" = 'prod' ]]; then
  echo "STOP"
  exit 1
fi

if [[ ! -f "$backup_filepath" ]]; then
  echo "No such file: $backup_filepath"
  exit 1
fi

echo "STARTING RESTORE TO [$deploy_env] OF [$backup_filepath]"

read -r -p "... THIS IS REALLY DANGEROUS, PROCEED? (Y/N): " answer

if [[ "$(tr '[:upper:]' '[:lower:]' <<< "$answer")" != "y" ]]; then
  echo "... EXITING"
  exit 1
fi

echo "... RESTORING"

PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/postgres_password.secret")" psql \
  --host "$ANT_DATA_FARM_HOST" \
  --port "$ANT_DATA_FARM_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/postgres_user.secret")" \
  --dbname template1 \
  --file "$backup_filepath" \
  --echo-all \
  | tee -a "$backup_filepath-restore.log"
