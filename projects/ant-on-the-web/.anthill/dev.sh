#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"

cargo run -p ant-on-the-web
