ips=($(hostname -I))
ipv4=${ips[0]}
ipv6=${ips[4]}

curl -4 $ipv4
curl -6 "[$ipv6]"
