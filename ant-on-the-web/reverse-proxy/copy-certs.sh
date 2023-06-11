#!/bin/bash

domain='beta.typesofants.org'
cert_path="/etc/letsencrypt/live/$domain"
data_path="./data/certbot/conf/live/$domain"

mkdir -p "$data_path"
cp "$cert_path/*" "$data_path/"
