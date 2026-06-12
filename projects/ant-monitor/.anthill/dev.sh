#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

cd "$repository_root/projects/ant-monitor" || exit 1
exec ./run.sh
