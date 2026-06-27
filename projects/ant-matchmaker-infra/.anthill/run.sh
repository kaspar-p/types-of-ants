#!/bin/bash

set -euo pipefail

exec "${BIN:-./consul}" agent \
  -config-format=hcl \
  -config-file=./consul.hcl \
  -config-file="$TYPESOFANTS_SECRET_DIR/infra_consul_gossip_encryption_key.secret" \
  -config-dir=./conf.d \
  -data-dir="$PERSIST_DIR/consul-data" \
  -http-port="$ANT_MATCHMAKER_INFRA_HTTP_PORT" \
  -serf-lan-port="$ANT_MATCHMAKER_INFRA_GOSSIP_PORT" \
  -server-port="$ANT_MATCHMAKER_INFRA_SERVER_PORT" \
  $ANT_MATCHMAKER_INFRA_SERVER_FLAGS \
  $ANT_MATCHMAKER_INFRA_RETRY_JOIN_FLAGS
