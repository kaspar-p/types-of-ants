#!/bin/bash

set -euo pipefail

deploy_env="$1"

repository_root="$(git rev-parse --show-toplevel)"

"$repository_root/scripts/migrate.sh" \
  ant-backing-it-up-db \
  ant_backing_it_up_db_password \
  ant_backing_it_up_db_user \
  ant_backing_it_up_db_db \
  ANT_BACKING_IT_UP_DB_HOST \
  ANT_BACKING_IT_UP_DB_PORT \
  "$deploy_env"
