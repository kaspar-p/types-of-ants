[Unit]
Description=The typesofants host agent!

[Service]
Type=simple
EnvironmentFile=.env
ExecStart=ant-host-agent
WorkingDirectory={{HOME}}/service/ant-host-agent//{{VERSION}}
Restart=always

[Install]
WantedBy=multi-user.target