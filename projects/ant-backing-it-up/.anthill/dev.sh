#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-backing-it-up"
cargo build --bin ant-backing-it-up

export BIN="$repository_root/target/debug/ant-backing-it-up"
exec "$repository_root/projects/ant-backing-it-up/.anthill/run.sh"
