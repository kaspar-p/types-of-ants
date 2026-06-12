#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-on-the-web"
cargo build --bin ant-on-the-web

export BIN="$repository_root/target/debug/ant-on-the-web"
exec "$repository_root/projects/ant-on-the-web/.anthill/run.sh"
