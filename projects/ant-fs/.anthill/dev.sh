#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cargo build --bin ant-fs

export BIN="$repository_root/target/debug/ant-fs"
exec "$repository_root/projects/ant-fs/.anthill/run.sh"
