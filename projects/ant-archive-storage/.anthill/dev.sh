#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-archive-storage/dev-fs"

cd "$repository_root/projects/ant-archive-storage"
cargo build --bin ant-archive-storage

export BIN="$repository_root/target/debug/ant-archive-storage"
exec ./.anthill/run.sh
