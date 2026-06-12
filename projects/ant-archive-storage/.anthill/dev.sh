#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-archive-storage"
cargo build --bin ant-archive-storage

export BIN="$repository_root/target/debug/ant-archive-storage"
exec "$repository_root/projects/ant-archive-storage/.anthill/run.sh"
