#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-printing-press/dev-fs/"

cd "$repository_root/projects/ant-printing-press"
cargo build --bin ant-printing-press

export BIN="$repository_root/target/debug/ant-printing-press"
exec "$repository_root/projects/ant-printing-press/.anthill/run.sh"
