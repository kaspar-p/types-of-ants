#!/bin/bash

set -euxo pipefail

repository_root="$(git rev-parse --show-toplevel)"

# Usage: ah dev ant-matchmaker [node:1/N | node:2/N | node:3/N]
# Omitting the arg (or node:1/1) runs a single-node cluster that immediately elects itself leader.
NODE_VARIANT="${1:-single}"

export BIN="consul"
export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-matchmaker/dev-fs/consul-data"

if [[ "$NODE_VARIANT" == "single" || "$NODE_VARIANT" == "node:1/1" ]]; then
  echo "Starting single-node Consul cluster..."
  export ANT_MATCHMAKER_SERVER_FLAGS="-server -node=local-node1 -bootstrap-expect=1"
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS=""

elif [[ "$NODE_VARIANT" == "client" ]]; then
  echo "Starting prod-connected client agent (port ${ANT_MATCHMAKER_CLIENT_HTTP_PORT})..."
  export TYPESOFANTS_SECRET_DIR="$repository_root/projects/ant-zookeeper/dev-fs/dev-fs/secrets-db/prod"
  export ANT_MATCHMAKER_HTTP_PORT="$ANT_MATCHMAKER_CLIENT_HTTP_PORT"
  export ANT_MATCHMAKER_GOSSIP_PORT="$ANT_MATCHMAKER_CLIENT_GOSSIP_PORT"
  export ANT_MATCHMAKER_SERVER_PORT="$ANT_MATCHMAKER_CLIENT_SERVER_PORT"
  export ANT_MATCHMAKER_SERVER_FLAGS="-node=$(hostname)"
  PERSIST_DIR="$(dirname "$PERSIST_DIR")/consul-data-client"
  export PERSIST_DIR

elif [[ "$NODE_VARIANT" == "node:1/3" ]]; then
  echo "Starting 3-node cluster: node 1 of 3"
  export ANT_MATCHMAKER_SERVER_FLAGS="-server -node=local-node1 -bootstrap-expect=3"
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS=""

elif [[ "$NODE_VARIANT" == "node:2/3" ]]; then
  echo "Starting 3-node cluster: node 2 of 3"
  export ANT_MATCHMAKER_SERVER_FLAGS="-server -node=local-node2 -bootstrap-expect=3"
  export ANT_MATCHMAKER_GOSSIP_PORT="9991"
  export ANT_MATCHMAKER_HTTP_PORT="9992"
  export ANT_MATCHMAKER_SERVER_PORT="9993"
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS="-retry-join 127.0.0.1:$ANT_MATCHMAKER_GOSSIP_PORT"
  PERSIST_DIR="$(dirname "$PERSIST_DIR")/consul-data2"
  export PERSIST_DIR

elif [[ "$NODE_VARIANT" == "node:3/3" ]]; then
  echo "Starting 3-node cluster: node 3 of 3"
  export ANT_MATCHMAKER_SERVER_FLAGS="-server -node=local-node3 -bootstrap-expect=3"
  export ANT_MATCHMAKER_GOSSIP_PORT="9994"
  export ANT_MATCHMAKER_HTTP_PORT="9995"
  export ANT_MATCHMAKER_SERVER_PORT="9996"
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS="-retry-join 127.0.0.1:$ANT_MATCHMAKER_GOSSIP_PORT"
  PERSIST_DIR="$(dirname "$PERSIST_DIR")/consul-data3"
  export PERSIST_DIR

else
  echo "Usage: ah dev ant-matchmaker [client | single | node:1/1 | node:1/3 | node:2/3 | node:3/3]"
  exit 1
fi

exec "$repository_root/projects/ant-matchmaker/.anthill/run.sh"
