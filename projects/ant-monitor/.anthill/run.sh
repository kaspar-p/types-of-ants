#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec "${BIN:-./prometheus}" \
  --web.enable-admin-api \
  --web.listen-address ":${ANT_MONITOR_PRIMARY_PORT}" \
  --storage.tsdb.retention.time=30d \
  --storage.tsdb.path="$PERSIST_DIR/prometheus-data"
