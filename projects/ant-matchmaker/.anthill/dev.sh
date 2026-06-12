#!/bin/bash

set -euxo pipefail

repository_root="$(git rev-parse --show-toplevel)"

function usage() {
  echo "USAGE: $0 <1|2|3>
  Where the 1, 2, or 3 denotes a node number, which listen for eachother to form a quorum, to test gossip
"
  exit 1
}

export NODE_VARIANT="$1"

export BIN="consul"
export TYPESOFANTS_SECRET_DIR="$repository_root/secrets/dev"
export PERSIST_DIR="$repository_root/projects/ant-matchmaker/dev-fs/consul-data"

if [[ "$NODE_VARIANT" == '1' ]]; then
  echo "continuing with default config..."
elif [[ "$NODE_VARIANT" == '2' ]]; then
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS="-retry-join 127.0.0.1:$ANT_MATCHMAKER_GOSSIP_PORT" # Other node's port
  export ANT_MATCHMAKER_GOSSIP_PORT="9991"
  export ANT_MATCHMAKER_HTTP_PORT="9992"
  export ANT_MATCHMAKER_SERVER_PORT="9993"

  PERSIST_DIR="$(dirname "$PERSIST_DIR")/consul-data2"
  export PERSIST_DIR
  export ANT_MATCHMAKER_SERVER_FLAGS="$ANT_MATCHMAKER_SERVER_FLAGS -node=local-node2"
elif [[ "$NODE_VARIANT" == '3' ]]; then
  export ANT_MATCHMAKER_RETRY_JOIN_FLAGS="-retry-join 127.0.0.1:$ANT_MATCHMAKER_GOSSIP_PORT" # Other node's port
  export ANT_MATCHMAKER_GOSSIP_PORT="9994"
  export ANT_MATCHMAKER_HTTP_PORT="9995"
  export ANT_MATCHMAKER_SERVER_PORT="9996"

  PERSIST_DIR="$(dirname "$PERSIST_DIR")/consul-data3"
  export PERSIST_DIR
  export ANT_MATCHMAKER_SERVER_FLAGS="$ANT_MATCHMAKER_SERVER_FLAGS -node=local-node3"
else
  usage
fi


exec "$repository_root/projects/ant-matchmaker/.anthill/run.sh"
