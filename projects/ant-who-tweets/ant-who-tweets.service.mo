[Unit]
Description=The @typesofants twitter bot!

[Service]
Type=simple
EnvironmentFile={{HOME}}/service/ant-who-tweets/{{VERSION}}/.env
ExecStart={{HOME}}/service/ant-who-tweets/{{VERSION}}/ant-who-tweets
WorkingDirectory={{HOME}}/service/ant-who-tweets
Restart=always

[Install]
WantedBy=multi-user.target
