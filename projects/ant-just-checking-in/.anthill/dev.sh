#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-just-checking-in"

export BUILD_OUTPUT_DIR='./build/.dev-tmp'
export PROMETHEUS_OS='darwin'
export PROMETHEUS_ARCH='arm64'

make release
export BIN="$BUILD_OUTPUT_DIR/blackbox_exporter"

exec ./.anthill/run.sh
