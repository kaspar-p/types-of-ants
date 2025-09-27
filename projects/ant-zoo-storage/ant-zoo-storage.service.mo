[Unit]
Description=A database for managing the ants...

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} up --no-build --force-recreate ant-zoo-storage
ExecStop=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} stop ant-zoo-storage
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
