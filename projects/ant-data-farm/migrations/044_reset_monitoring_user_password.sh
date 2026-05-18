#!/bin/bash

set -euo pipefail

DB_USER_FILEPATH="${1:-/run/secrets/ant_data_farm_user}"
DB_NAME_FILEPATH="${2:-/run/secrets/ant_data_farm_db}"
MONITORING_PASSWORD_FILEPATH="${3:-/run/secrets/ant_db_monitoring_password}"
HOST="${4:-localhost}"
PORT="${5:-5432}"

echo "Reading monitoring password..."
MONITORING_PW="$(cat "$MONITORING_PASSWORD_FILEPATH")"

echo "Resetting monitoring user's password..."

MIGRATION="
BEGIN;

ALTER USER monitoring WITH PASSWORD '$MONITORING_PW';
INSERT INTO migration (migration_label) VALUES ('reset-monitoring-user-pw');

COMMIT;
"

psql -c "$MIGRATION" \
  --host "$HOST" \
  --port "$PORT" \
  --username "$(cat "$DB_USER_FILEPATH")" \
  --dbname "$(cat "$DB_NAME_FILEPATH")"
