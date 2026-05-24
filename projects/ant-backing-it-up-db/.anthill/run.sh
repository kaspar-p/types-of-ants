#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-backing-it-up-db \
  up \
  --no-build \
  --force-recreate \
  ant-backing-it-up-db
