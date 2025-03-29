#!/bin/bash

#
# Initialize the ddclient program on a new host.
#   Only needs to be running on a single host in the fleet ddclient is a
#   program that updates the DNS record for beta.typesofants.org based on the
#   public IP address of the home router.
#
#   See ./production-guide.md for more on ddclient.
#

set -euo pipefail

function usage() {
  echo "$0 <env-file-path>"
  echo "   env-file-path: A .env file with a CLOUDFLARE_API_TOKEN variable."
  exit 1
}

if [[ -z "$1" ]]; then
  usage
fi

echo '[INFO] COPYING DDCLIENT CONFIG...'

ENV_FILE_PATH="$1"
set -o allexport
source "$ENV_FILE_PATH"
set +o allexport

set -x

TEMPLATE_FILE="$(dirname $0)//ddclient.conf.mo"
mo "$TEMPLATE_FILE" > /etc/ddclient/ddclient.conf

echo '[INFO] STARTING DDCLIENT...'

ddclient -daemon=0 -debug -verbose -noquiet

ps aux | grep ddclient | grep sleeping

echo "DDCLIENT RUNNING AND HEALTHY"
