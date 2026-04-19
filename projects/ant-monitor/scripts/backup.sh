#!/bin/bash

set -euo pipefail

res="$(curl \
  --no-progress-meter \
  --fail-with-body \
  -X POST \
  http://localhost:9090/api/v1/admin/tsdb/snapshot)"

status="$(jq -r '.status' <<< "$res")"
if [[ "$status" != "success" ]]; then
  echo "Failed to take backup: $(jq <<< "$res")"
  exit 1
fi

backup_id="$(jq -r '.data.name' <<< "$res")"

echo "Successfully made backup with ID: $backup_id"
