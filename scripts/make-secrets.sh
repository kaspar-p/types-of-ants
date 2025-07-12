#!/bin/bash

set -euxo pipefail

deploy_env="$1"

root="$(git rev-parse --show-toplevel)"
cd "$root"

for file in $(find "secrets/$deploy_env" -type f -name "*.env");
do
  cat "$file"
done
