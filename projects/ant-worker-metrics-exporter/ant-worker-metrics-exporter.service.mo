[Unit]
Description=The typesofants per-host metrics exporter!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR={{INSTALL_DIR}}/secrets
ExecStart={{INSTALL_DIR}}/node_exporter
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
