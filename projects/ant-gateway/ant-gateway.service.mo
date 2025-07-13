[Unit]
Description=The typesofants reverse proxy!

[Service]
Type=simple
Restart=always
ExecStart=VERSION={{VERSION}} ANT_ON_THE_WEB_WORKER_NUM={{ANT_ON_THE_WEB_WORKER_NUM}} WEBSERVER_PORT={{WEBSERVER_PORT}} /snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml up --build ant-gateway
ExecStop=VERSION={{VERSION}} ANT_ON_THE_WEB_WORKER_NUM={{ANT_ON_THE_WEB_WORKER_NUM}} WEBSERVER_PORT={{WEBSERVER_PORT}} /snap/bin/docker-compose --project-directory {{HOME}}/types-of-ants/ --file {{HOME}}/types-of-ants/docker-compose.yml stop --build ant-gateway
EnvironmentFile={{HOME}}/service/ant-gateway/{{VERSION}}/.env

[Install]
WantedBy=multi-user.target
