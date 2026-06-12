#!/bin/bash

set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"

exec "$repository_root/projects/ant-monitor/run.sh"
