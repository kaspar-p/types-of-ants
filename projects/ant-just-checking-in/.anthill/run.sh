#!/bin/bash

set -euo pipefail

exec "${BIN:-./blackbox_exporter}" \
  --config.file="./blackbox.yml" \
  --web.listen-address=":$PORT"
