[Unit]
Description=The @typesofants twitter bot!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR=%d
ExecStart={{INSTALL_DIR}}/ant-who-tweets
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
