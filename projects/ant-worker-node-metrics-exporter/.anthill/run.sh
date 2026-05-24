#!/bin/bash

set -euo pipefail

exec "${BIN:-./node_exporter}" \
  --web.listen-address=":$PORT" \
  --collector.systemd
