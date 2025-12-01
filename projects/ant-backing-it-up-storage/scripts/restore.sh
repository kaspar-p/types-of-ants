#!/bin/bash

set -euo pipefail

deploy_env="$1"
backup_filepath="$2"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

if [[ ! -f "$backup_filepath" ]]; then
  echo "No such file: $backup_filepath" >> /dev/stderr
  exit 1
fi

echo "STARTING RESTORE TO [$deploy_env] OF [$backup_filepath]" >> /dev/stderr

read -r -p "... THIS IS REALLY DANGEROUS, PROCEED? (Y/N): " answer

if [[ "$(tr '[:upper:]' '[:lower:]' <<< "$answer")" != "y" ]]; then
  echo "... EXITING" >> /dev/stderr
  exit 1
fi

echo "... RESTORING" >> /dev/stderr 

PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/ant_backing_it_up_storage_password.secret")" psql \
  --host "$ANT_BACKING_IT_UP_STORAGE_HOST" \
  --port "$ANT_BACKING_IT_UP_STORAGE_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/ant_backing_it_up_storage_user.secret")" \
  --dbname template1 \
  --file "$backup_filepath" \
  --echo-all \
  | tee -a "$backup_filepath-restore.log" >> /dev/stderr 
