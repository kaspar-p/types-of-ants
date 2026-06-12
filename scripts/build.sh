#!/bin/bash

#
# A script to build a deployment.tar file, as per the docs/design/deployment-manifest.md file specification.
# Builds both makefile-based or docker-based services.
#

set -euo pipefail

cargo run -p anthill -- build "${@}"
exit 0
