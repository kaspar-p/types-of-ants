#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-gateway \
  up \
  --no-build \
  --force-recreate \
  ant-gateway
