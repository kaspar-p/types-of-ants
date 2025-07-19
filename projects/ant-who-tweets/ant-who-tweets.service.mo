[Unit]
Description=The @typesofants twitter bot!

[Service]
Type=simple
EnvironmentFile={{INSTALL_DIR}}/.env
Environment=TYPESOFANTS_SECRET_DIR=%d
ExecStart={{INSTALL_DIR}}/ant-who-tweets
WorkingDirectory={{INSTALL_DIR}}
Restart=always
LoadCredentialEncrypted=twitter_consumer_key.secret: {{INSTALL_DIR}}/secrets/twitter_consumer_key.secret
LoadCredentialEncrypted=twitter_consumer_secret.secret: {{INSTALL_DIR}}/secrets/twitter_consumer_secret.secret
LoadCredentialEncrypted=twitter_access_token.secret: {{INSTALL_DIR}}/secrets/twitter_access_token.secret
LoadCredentialEncrypted=twitter_access_token_secret.secret: {{INSTALL_DIR}}/secrets/twitter_access_token_secret.secret
LoadCredentialEncrypted=postgres_db.secret: {{INSTALL_DIR}}/secrets/postgres_db.secret
LoadCredentialEncrypted=postgres_user.secret: {{INSTALL_DIR}}/secrets/postgres_user.secret
LoadCredentialEncrypted=postgres_password.secret: {{INSTALL_DIR}}/secrets/postgres_password.secret

[Install]
WantedBy=multi-user.target
