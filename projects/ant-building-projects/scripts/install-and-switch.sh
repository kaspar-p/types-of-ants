#!/bin/bash

set -euxo pipefail

project="$1"

GIT_COMMIT="$(git log --format='%h' -n 1)"
INSTALL_VERSION="$(date "+%Y-%m-%d@%H:%M@$GIT_COMMIT")"

# Build the project
make -C "../$project" release

# Install the project
make -C "../$project" install INSTALL_VERSION="$INSTALL_VERSION"

# Cut over to the systemd service
os="$(uname -s)"
SYSTEMD_DIR=""
if [[ "$os" = "Linux" ]]; then
  SYSTEMD_DIR="/etc/systemd/system/"
else
  echo "ERROR: Cannot install systemd service on non-linux machine."
  exit 1
fi

# Remove the current systemd service file
unit_path="${SYSTEMD_DIR:?}/$project.service"
rm -rf "$unit_path"
ln -s "$unit_path" "$HOME/service/$project/$INSTALL_VERSION/$project.service"

# Restart the service
sudo systemctl restart "$project.service"
