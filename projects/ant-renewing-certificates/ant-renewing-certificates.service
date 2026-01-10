[Unit]
Description=The typesofants certificate renewer!

[Service]
Type=oneshot
ExecStart={{INSTALL_DIR}}/getssl/getssl.sh -d -w {{INSTALL_DIR}}/getssl {{ANT_RENEWING_CERTIFICATES_FQDN}}
EnvironmentFile={{INSTALL_DIR}}/.env
WorkingDirectory={{INSTALL_DIR}}/getssl

[Timer]
OnUnitInactiveSec=12hours
RandomizedDelaySec=12hours
AccuracySec=1s

[Install]
WantedBy=default.target
