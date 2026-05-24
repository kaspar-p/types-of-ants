#!/bin/bash

set -euo pipefail

exec "${BIN:-./ant-who-tweets}"
