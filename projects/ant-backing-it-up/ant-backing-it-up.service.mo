[Unit]
Description=The typesofants backup processor!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR={{INSTALL_DIR}}/secrets
ExecStart={{INSTALL_DIR}}/ant-backing-it-up
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
