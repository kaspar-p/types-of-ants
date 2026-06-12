#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
secrets_dir="$repository_root/secrets/dev"

# Use the embedded PG binaries if psql isn't already in PATH
if ! command -v psql &>/dev/null; then
  export PATH="$repository_root/build/pgtmp/18.3.0/bin:$PATH"
fi

cargo build --bin ant-archive

# Seed the dev database with a KEK, client, buckets, and storage node.
# Idempotent — safe to re-run.
DEV_BEARER_TOKEN="dev-bearer-token"
TOKEN_HASH=$(printf '%s' "$DEV_BEARER_TOKEN" | openssl dgst -sha256 -binary | base64)

PGPASSWORD="$(cat "$secrets_dir/ant_archive_db_password.secret")" \
psql \
  --host="$ANT_ARCHIVE_DB_HOST" \
  --port="$ANT_ARCHIVE_DB_PORT" \
  --username="$(cat "$secrets_dir/ant_archive_db_user.secret")" \
  --dbname="$(cat "$secrets_dir/ant_archive_db_db.secret")" \
  --no-psqlrc \
  <<SQL
insert into archive_kek_version (kek_id, is_active)
values ('kek-dev', true)
on conflict (kek_id) do nothing;

insert into archive_storage_node (storage_node_id, host_id, is_active)
values ('sn-dev', 'local-node1', true)
on conflict (storage_node_id) do nothing;

insert into archive_client (client_id, client_name, token_hash)
values ('client-dev', 'dev', '$TOKEN_HASH')
on conflict (client_id) do nothing;

insert into archive_bucket (bucket_id, client_id, is_default, read_policy)
values ('b-dev', 'client-dev', true, 'private')
on conflict (bucket_id) do nothing;

insert into archive_bucket (bucket_id, client_id, is_default, read_policy)
values ('b-public', 'client-dev', false, 'public')
on conflict (bucket_id) do nothing;
SQL

echo "Dev database seeded. Bearer token: $DEV_BEARER_TOKEN"

export BIN="$repository_root/target/debug/ant-archive"
exec "$repository_root/projects/ant-archive/.anthill/run.sh"
