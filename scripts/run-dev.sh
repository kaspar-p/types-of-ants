#!/bin/bash

set -euo pipefail

cargo run -p anthill -- dev "${@}"
exit 0
