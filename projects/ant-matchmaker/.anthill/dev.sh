#!/bin/bash

repository_root="$(git rev-parse --show-toplevel)"

export BIN="consul"
export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-matchmaker/dev-fs/consul-data"

cd "$repository_root/projects/ant-matchmaker" || exit 1
exec "$repository_root/projects/ant-matchmaker/.anthill/run.sh"
