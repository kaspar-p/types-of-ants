#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-host-agent"
cargo build --bin ant-host-agent

export BIN="$repository_root/target/debug/ant-host-agent"
exec "$repository_root/projects/ant-host-agent/.anthill/run.sh"
