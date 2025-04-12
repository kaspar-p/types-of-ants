[Unit]
Description=The @typesofants twitter bot!

[Service]
Type=simple
EnvironmentFile=.env
ExecStart=ant-who-tweets
WorkingDirectory={{HOME}}/service/ant-who-tweets
Restart=always

[Install]
WantedBy=multi-user.target
