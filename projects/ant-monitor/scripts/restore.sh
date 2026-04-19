#!/bin/bash

set -euo pipefail

echo "1/ Turn off existing Prometheus instance"
echo "2/ Find existing backup ID <id>"
echo "3/ Copy <data-dir>/snapshots/<id>/* into <data-dir>"
