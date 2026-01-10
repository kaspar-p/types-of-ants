[Unit]
Description=The typesofants database!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} up --no-build --force-recreate ant-data-farm
ExecStop=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} stop ant-data-farm
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
