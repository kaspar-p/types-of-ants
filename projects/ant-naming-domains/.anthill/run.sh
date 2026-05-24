#!/bin/bash

set -euo pipefail

echo "Templating ddclient.conf..."
ANT_NAMING_DOMAINS_FQDN="$ANT_NAMING_DOMAINS_FQDN" "${MO_BIN:-/home/ant/installs/mo}" ./config/ddclient.conf.mo > ./config/ddclient.conf

echo "Starting service..."
exec /snap/bin/docker-compose \
  --project-name ant-naming-domains \
  up \
  --no-build \
  --force-recreate \
  ant-naming-domains
