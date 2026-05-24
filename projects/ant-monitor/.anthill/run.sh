#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec "${BIN:-prometheus}" \
  --web.enable-admin-api \
  --storage.tsdb.retention.time=30d \
  --storage.tsdb.path="$PERSIST_DIR/prometheus-data"
