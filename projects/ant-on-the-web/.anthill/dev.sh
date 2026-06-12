#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cargo build --bin ant-on-the-web

export BIN="$repository_root/target/debug/ant-on-the-web"
exec "$repository_root/projects/ant-on-the-web/.anthill/run.sh"
