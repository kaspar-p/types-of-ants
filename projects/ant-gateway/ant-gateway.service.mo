[Unit]
Description=The typesofants reverse proxy!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} up --no-build ant-gateway
ExecStop=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} stop --no-build ant-gateway
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
