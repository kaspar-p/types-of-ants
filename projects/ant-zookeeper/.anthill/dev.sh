#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-zookeeper" || exit 1
cargo build --bin ant-zookeeper

export BIN="$repository_root/target/debug/ant-zookeeper"
exec "$repository_root/projects/ant-zookeeper/.anthill/run.sh"
