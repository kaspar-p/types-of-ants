[Unit]
Description=The typesofants reverse proxy!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} up --no-build --force-recreate ant-gateway
ExecStop=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} stop ant-gateway
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}
Restart=always

LoadCredentialEncrypted=tls_cert.pem:{{INSTALL_DIR}}/secrets/tls_cert.pem
LoadCredentialEncrypted=tls_key.pem:{{INSTALL_DIR}}/secrets/tls_key.pem

[Install]
WantedBy=multi-user.target
