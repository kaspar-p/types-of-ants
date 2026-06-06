#!/bin/bash

#
# A script to build a deployment.tar file, as per the docs/design/deployment-manifest.md file specification.
# Builds both makefile-based or docker-based services.
#

# shellcheck disable=SC1091
source "$(git rev-parse --show-toplevel)/scripts/lib.sh"

set -euo pipefail

cargo run -p anthill -- build "${@}"
exit 0
