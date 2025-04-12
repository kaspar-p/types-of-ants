[Unit]
Description=The reverse proxy for typesofants.org!

[Service]
Type=simple
ExecStart=/bin/bash -c "docker-compose -f {{HOME}}/types-of-ants/docker-compose.yml up --build ant-gateway"
ExecStop=/bin/bash -c "docker-compose -f {{HOME}}/types-of-ants/docker-compose.yml stop --build ant-gateway"

[Install]
WantedBy=multi-user.target
