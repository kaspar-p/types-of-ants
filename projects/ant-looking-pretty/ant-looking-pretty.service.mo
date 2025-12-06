[Unit]
Description=The typesofants frontend!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR={{INSTALL_DIR}}/secrets
ExecStart=/home/ant/.nvm/versions/node/current {{INSTALL_DIR}}/server.js
WorkingDirectory={{INSTALL_DIR}}
Restart=always

[Install]
WantedBy=multi-user.target
