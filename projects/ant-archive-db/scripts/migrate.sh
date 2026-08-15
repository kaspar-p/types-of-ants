#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

exec "$repository_root/scripts/migrate.sh" "ant-archive-db" "$deploy_env"
