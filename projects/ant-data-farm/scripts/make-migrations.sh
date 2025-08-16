#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

current_highest_migration="$(
  PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/postgres_password.secret")" psql \
  --host "$ANT_DATA_FARM_HOST" \
  --port "$ANT_DATA_FARM_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/postgres_user.secret")" \
  --dbname "$(cat "$repository_root/secrets/$deploy_env/postgres_db.secret")" \
  --tuples-only \
  --no-align \
  --command='select max(migration_seq) from migration;'
)"

migrations_root="$repository_root/projects/ant-data-farm/migrations"
for filename in "$migrations_root"/*; do
  migration_num=$(basename "$filename" | cut -d '_' -f 1)
  if [ "$migration_num" -le "$current_highest_migration" ]; then
    echo "... skipping $filename"
    continue
  fi

  echo "... applying $filename"
  PGPASSWORD="$(cat "$repository_root/secrets/$deploy_env/postgres_password.secret")" psql \
  --host "$ANT_DATA_FARM_HOST" \
  --port "$ANT_DATA_FARM_PORT" \
  --username "$(cat "$repository_root/secrets/$deploy_env/postgres_user.secret")" \
  --dbname "$(cat "$repository_root/secrets/$deploy_env/postgres_db.secret")" \
  --file "$filename" \
  --echo-all
done
