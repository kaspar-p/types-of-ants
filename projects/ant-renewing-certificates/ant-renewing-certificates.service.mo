[Unit]
Description=The typesofants certificate renewer!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} up --no-build --force-recreate ant-renewing-certificates
ExecStop=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} down ant-renewing-certificates
Restart=always

[Install]
WantedBy=default.target
