[Unit]
Description=The typesofants dynamic dns client!

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} up --no-build --force-recreate ant-naming-domains
ExecStop=/snap/bin/docker-compose --project-directory {{INSTALL_DIR}} down ant-naming-domains
Restart=always

[Install]
WantedBy=default.target
