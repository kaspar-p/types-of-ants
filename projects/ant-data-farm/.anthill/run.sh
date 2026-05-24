#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-data-farm \
  up \
  --no-build \
  --force-recreate \
  ant-data-farm
