#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export SECRETS_DIR="$repository_root/secrets/dev"
export VERSION=dev
export PERSIST_DIR="$repository_root/projects/ant-backing-it-up-storage/database-files"

docker-compose up --build ant-backing-it-up-storage "${@:2}"
