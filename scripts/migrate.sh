#!/bin/bash

set -euo pipefail

project="$1"
password_secret_name="$2"
username_secret_name="$3"
db_secret_name="$4"
env_host_name="$5"
env_port_name="$6"
monitoring_password_secret_name="ant_db_monitoring_password"
deploy_env="$7"

repository_root="$(git rev-parse --show-toplevel)"

# shellcheck disable=SC1091
source "$repository_root/scripts/lib.sh"

set +o allexport
# shellcheck disable=SC1090
source "$(deploy_env_filepath "$deploy_env")"
set -o allexport

secrets_dir="$(find_secrets_dir "$deploy_env")"

monitoring_password_filepath="$secrets_dir/$monitoring_password_secret_name.secret"
username_filepath="$secrets_dir/$username_secret_name.secret"
username="$(cat "$username_filepath")"
password_filepath="$secrets_dir/$password_secret_name.secret"
password="$(cat "$password_filepath")"
dbname_filepath="$secrets_dir/$db_secret_name.secret"
dbname="$(cat "$dbname_filepath")"

current_highest_migration="$(
  PGPASSWORD="$password" psql \
    --host "${!env_host_name}" \
    --port "${!env_port_name}" \
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
      --host "${!env_host_name}" \
      --port "${!env_port_name}" \
      --username "$username" \
      --dbname "$dbname" \
      --file "$filepath" \
      --echo-all | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
  elif [[ "$extension" == "sh" ]]; then
    PGPASSWORD="$password" "$filepath" "$username_filepath" "$dbname_filepath" "$monitoring_password_filepath" "${!env_host_name}" "${!env_port_name}" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
  else
    echo "unknown extension" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
    exit 1
  fi
done

echo "applied all migrations" | tee -a "$repository_root/logs/migration.log" >> /dev/stderr
