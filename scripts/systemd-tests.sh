#!/bin/bash

set -euxo pipefail

deployment_tarball_path="$1"

scp "$deployment_tarball_path" ant@antworker007.hosts.typesofants.org:/tmp/ant-host-agent.test.tar.gz

ssh2ant 007 "
  mkdir -p /tmp/ant-host-agent.test && 
  cd /tmp/ant-host-agent.test && 
  tar -xvzf /tmp/ant-host-agent.test.tar.gz
"

ssh2ant 007 '
echo "
PERSIST_DIR=/tmp/ant-host-agent.test/persist
VERSION=test
ANT_HOST_AGENT_INSTALL_ROOT_DIR=./install
PORT=3333
" > /tmp/ant-host-agent.test/.env
'