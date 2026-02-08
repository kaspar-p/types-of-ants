#!/bin/bash

set -euo pipefail

cd projects/ant-zookeeper-ui

npm ci
npm run dev
