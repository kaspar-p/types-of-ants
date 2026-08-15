#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

export BIN="prometheus"
exec "$repository_root/projects/ant-monitor/.anthill/run.sh"
