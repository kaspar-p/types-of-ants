#!/bin/bash

set -euo pipefail

project="$1"
deploy_env="$2"

repository_root="$(git rev-parse --show-toplevel)"

anthill="$(cat "$repository_root/projects/$project/anthill.json")"

password_secret_name="$(jq -r .archetype.postgres.password_secret_name <<< "$anthill")"
username_secret_name="$(jq -r .archetype.postgres.username_secret_name <<< "$anthill")"
db_secret_name="$(jq -r .archetype.postgres.database_secret_name <<< "$anthill")"

monitoring_password_secret_name="ant_db_monitoring_password"

# shellcheck disable=SC1091
source "$repository_root/scripts/lib.sh"

set +o allexport
# shellcheck disable=SC1090
source "$(deploy_env_filepath "$deploy_env")"
set -o allexport

secrets_dir="$(find_secrets_dir "$deploy_env")"

consul_res="$(curl "http://localhost:9990/v1/catalog/service/$project")"

echo "$consul_res" | jq

ip="$(echo "$consul_res" | jq -r '.[0].Address')"
port="$(echo "$consul_res" | jq -r '.[0].ServicePort')"

monitoring_password_filepath="$secrets_dir/$monitoring_password_secret_name.secret"
username_filepath="$secrets_dir/$username_secret_name.secret"
username="$(cat "$username_filepath")"
password_filepath="$secrets_dir/$password_secret_name.secret"
password="$(cat "$password_filepath")"
dbname_filepath="$secrets_dir/$db_secret_name.secret"
dbname="$(cat "$dbname_filepath")"

current_highest_migration="$(
  PGPASSWORD="$password" psql \
    --host "$ip" \
    --port "$port" \
    --username "$username" \
    --dbname "$dbname" \
    --tuples-only \
    --no-align \
    --command='select max(migration_seq) from migration;'
)"

echo "highest applied migration: $current_highest_migration" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr

set -euo pipefail

migrations_root="$repository_root/projects/$project/migrations"
for filepath in "$migrations_root"/*; do
  migration_num="$(basename "$filepath" | cut -d '_' -f 1 | cut -d '-' -f 1)"
  if [ "$migration_num" -le "$current_highest_migration" ]; then
    echo "... skipping $filepath" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
    continue
  fi

  extension="$(basename "$filepath")"
  extension="${extension##*.}"

  echo "... applying $filepath" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
  if [[ "$extension" == "sql" ]]; then
    PGPASSWORD="$password" psql \
      --host "$ip" \
      --port "$port" \
      --username "$username" \
      --dbname "$dbname" \
      --file "$filepath" \
      --echo-all | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
  elif [[ "$extension" == "sh" ]]; then
    PGPASSWORD="$password" "$filepath" "$username_filepath" "$dbname_filepath" "$monitoring_password_filepath" "$ip" "$port" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
  else
    echo "unknown extension" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
    exit 1
  fi
done

echo "applied all migrations" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
