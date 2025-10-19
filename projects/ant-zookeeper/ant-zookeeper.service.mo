[Unit]
Description=The typesofants process manager!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR={{INSTALL_DIR}}/secrets
ExecStart={{INSTALL_DIR}}/ant-zookeeper
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
