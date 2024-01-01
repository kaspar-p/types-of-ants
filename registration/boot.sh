#!/bin/bash

cd ~

# Install ddclient
sudo apt-get install ddclient
DDCLIENT_CONF='/etc/ddclient/ddclient.conf'
sudo touch "$DDCLIENT_CONF"
echo '# ddclient for typesofants.org' >> "$DDCLIENT_CONF"
echo '# /etc/ddclient/ddclient.conf' >> "$DDCLIENT_CONF"
echo 'protocol=googledomains' >> "$DDCLIENT_CONF"
echo 'use=web' >> "$DDCLIENT_CONF"
echo 'login=' >> "$DDCLIENT_CONF"
echo 'password=' >> "$DDCLIENT_CONF"
echo 'beta.typesofants.org'

# Run ddclient in the background
sudo ddclient

# Somehow only clone the necessary HTML files? Why clone everything?
REPO_NAME='types-of-ants'
git clone "https://github.com/kaspar-p/$REPO_NAME.git"

# Delete everything except the index.html file inside the repository directory
mv "$REPO_NAME"/index.html index.html
rm -rf "$REPO_NAME"
mkdir "$REPO_NAME"
mv index.html "$REPO_NAME"/index.html

# Point nginx to that directory
