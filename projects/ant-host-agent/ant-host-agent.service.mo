[Unit]
Description=The typesofants host agent!

[Service]
Type=simple
EnvironmentFile={{HOME}}/service/ant-host-agent/{{VERSION}}/.env
ExecStart={{HOME}}/service/ant-host-agent/{{VERSION}}/ant-host-agent
WorkingDirectory={{HOME}}/service/ant-host-agent/{{VERSION}}
Restart=always

[Install]
WantedBy=multi-user.target
