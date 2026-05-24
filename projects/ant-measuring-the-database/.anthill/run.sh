#!/bin/bash

set -euo pipefail

monitoring_user_password="$(cat "${TYPESOFANTS_SECRET_DIR}/ant_db_monitoring_password.secret")"

sed -i "/password: /c\      password: $monitoring_user_password" ./postgres_exporter.yml

exec "${BIN:-./postgres_exporter}" \
  --web.listen-address=":${PORT}"
