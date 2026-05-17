#!/bin/bash

set -euo pipefail

echo "Reading monitoring password..."
MONITORING_PW="$(cat /run/secrets/ant_db_monitoring_password)"

MIGRATION="
BEGIN;

ALTER USER monitoring WITH PASSWORD '$MONITORING_PW';
INSERT INTO migration (migration_label) VALUES ('reset-monitoring-user-pw');

COMMIT;
"

echo "Resetting monitoring user's password..."
psql -c "$MIGRATION" \
  -v MONITORING_PW="${MONITORING_PW}" \
  --username "$(cat /run/secrets/ant_zookeeper_db_user)" \
  --dbname "$(cat /run/secrets/ant_zookeeper_db_db)"
