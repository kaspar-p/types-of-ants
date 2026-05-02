#!/bin/bash

set -euo pipefail

deploy_env="dev"

repository_root="$(git rev-parse --show-toplevel)"

set +o allexport
# shellcheck disable=SC1090
source "$repository_root/secrets/$deploy_env/build.cfg"
set -o allexport

rm -rf "$repository_root/projects/ant-zookeeper-db/database-files"
