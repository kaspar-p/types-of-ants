[Unit]
Description=The typesofants web server!

[Service]
Type=simple
EnvironmentFile={{HOME}}/service/ant-on-the-web/{{VERSION}}/.env
ExecStart={{HOME}}/service/ant-on-the-web/{{VERSION}}/ant-on-the-web
WorkingDirectory={{HOME}}/service/ant-on-the-web/{{VERSION}}
Restart=always

[Install]
WantedBy=multi-user.target
