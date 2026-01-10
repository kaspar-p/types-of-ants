[Unit]
Description=A database for knowing about backups!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} up --no-build --force-recreate ant-backing-it-up-storage
ExecStop=/snap/bin/docker-compose --project-directory={{INSTALL_DIR}} stop ant-backing-it-up-storage
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
