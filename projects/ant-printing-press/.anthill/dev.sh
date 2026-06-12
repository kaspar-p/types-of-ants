#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cargo build --bin ant-printing-press

export BIN="$repository_root/target/debug/ant-printing-press"
exec "$repository_root/projects/ant-printing-press/.anthill/run.sh"
