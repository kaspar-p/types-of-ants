#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-monitor-fe \
  up \
  --no-build \
  --force-recreate \
  ant-monitor-fe
