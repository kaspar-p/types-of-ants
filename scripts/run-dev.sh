#!/bin/bash

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

cargo run -p anthill -- dev "${@}"
exit 0
