#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/projects/ant-zookeeper/dev-fs/dev-fs/secrets-db/prod"

cargo run
