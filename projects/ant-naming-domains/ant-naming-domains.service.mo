[Unit]
Description=The typesofants dynamic dns client!

[Service]
Type=simple
Restart=always
ExecStart=/snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml up ant-naming-domains
ExecStop=/snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml down ant-naming-domains

[Install]
WantedBy=default.target
