#!/bin/bash

set -euo pipefail

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-zookeeper-db \
  up \
  --no-build \
  --force-recreate \
  ant-zookeeper-db
