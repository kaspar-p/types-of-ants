#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-archive-db \
  up \
  --no-build \
  --force-recreate \
  ant-archive-db
