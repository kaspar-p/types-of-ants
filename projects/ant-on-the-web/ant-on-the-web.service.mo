[Unit]
Description=The typesofants web server!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR=%d
ExecStart={{INSTALL_DIR}}/ant-on-the-web
WorkingDirectory={{INSTALL_DIR}}
Restart=always

LoadCredentialEncrypted=postgres_db.secret:{{INSTALL_DIR}}/secrets/postgres_db.secret
LoadCredentialEncrypted=postgres_password.secret:{{INSTALL_DIR}}/secrets/postgres_password.secret
LoadCredentialEncrypted=postgres_user.secret:{{INSTALL_DIR}}/secrets/postgres_user.secret

LoadCredentialEncrypted=twilio_account_id.secret:{{INSTALL_DIR}}/secrets/twilio_account_id.secret
LoadCredentialEncrypted=twilio_auth_token.secret:{{INSTALL_DIR}}/secrets/twilio_auth_token.secret
LoadCredentialEncrypted=twilio_phone_number.secret:{{INSTALL_DIR}}/secrets/twilio_phone_number.secret

LoadCredentialEncrypted=mailjet_api_key.secret:{{INSTALL_DIR}}/secrets/mailjet_api_key.secret
LoadCredentialEncrypted=mailjet_secret_key.secret:{{INSTALL_DIR}}/secrets/mailjet_secret_key.secret

LoadCredentialEncrypted=jwt.secret:{{INSTALL_DIR}}/secrets/jwt.secret

[Install]
WantedBy=multi-user.target
