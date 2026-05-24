#!/bin/bash

set -euo pipefail

exec ${BIN:-./consul} agent \
  -config-format=hcl \
  -config-file=./consul.hcl \
  -config-file="$TYPESOFANTS_SECRET_DIR/consul_gossip_encryption_key.secret" \
  -data-dir="$PERSIST_DIR/consul-data" \
  -http-port="$ANT_MATCHMAKER_HTTP_PORT" \
  -serf-lan-port="$ANT_MATCHMAKER_GOSSIP_PORT" \
  -server-port="$ANT_MATCHMAKER_SERVER_PORT" \
  $ANT_MATCHMAKER_SERVER_FLAGS \
  $ANT_MATCHMAKER_RETRY_JOIN_FLAGS
