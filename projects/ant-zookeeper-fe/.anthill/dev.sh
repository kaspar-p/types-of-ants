#!/bin/bash

set -euo pipefail

cd projects/ant-zookeeper-fe

npm ci
npm run dev
