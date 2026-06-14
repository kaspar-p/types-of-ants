#!/bin/bash

set -euo pipefail

echo -n "$1" | sha256sum | cut -d ' ' -f 1 | xxd -r -p | base64
