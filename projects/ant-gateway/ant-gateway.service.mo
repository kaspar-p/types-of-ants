[Unit]
Description=The typesofants reverse proxy!

[Service]
Type=simple
Restart=always
ExecStart=/snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml up --build ant-gateway
ExecStop=/snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml stop --build ant-gateway

[Install]
WantedBy=multi-user.target
