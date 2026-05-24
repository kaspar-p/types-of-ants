#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-zookeeper/dev-fs"

cd "$repository_root/projects/ant-zookeeper" || exit 1

cargo run
