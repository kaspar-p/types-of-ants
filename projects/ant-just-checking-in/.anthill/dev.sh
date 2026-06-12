#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export BUILD_OUTPUT_DIR="$repository_root/projects/ant-just-checking-in/build/.dev-tmp"
export PROMETHEUS_OS='darwin'
export PROMETHEUS_ARCH='arm64'

make -C "$repository_root/projects/ant-just-checking-in" release
export BIN="$BUILD_OUTPUT_DIR/blackbox_exporter"

exec "$repository_root/projects/ant-just-checking-in/.anthill/run.sh"
