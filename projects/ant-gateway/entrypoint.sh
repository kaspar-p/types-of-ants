#!/usr/bin/env sh
set -eu

# shellcheck disable=SC2016
envsubst '${VERSION} ${ANT_ON_THE_WEB_NUM} ${FQDN} ${WEBSERVER_PORT}' \
  < /etc/nginx/conf.d/default.conf.template \
  > /etc/nginx/conf.d/default.conf

exec "$@"