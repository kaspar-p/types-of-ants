#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-backing-it-up/dev-fs"

cd "$repository_root/projects/ant-backing-it-up"
cargo run --bin ant-backing-it-up
