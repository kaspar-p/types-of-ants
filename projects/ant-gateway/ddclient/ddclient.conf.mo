daemon=300
syslog=yes
verbose=yes
pid=/var/run/ddclient.pid
ssl=yes
use=web
web='https://cloudflare.com/cdn-cgi/trace'
web-skip='ip='

protocol=cloudflare, \
zone=typesofants.org, \
ttl=1,
login=token,
password='{{CLOUDFLARE_API_TOKEN}}',
beta.typesofants.org