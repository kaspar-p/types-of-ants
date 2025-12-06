#!/bin/bash

set -euo pipefail

cd projects/ant-looking-pretty
npm ci
npm run dev
