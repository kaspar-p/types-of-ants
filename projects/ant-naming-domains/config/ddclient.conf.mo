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
password_env=CLOUDFLARE_API_TOKEN,
{{ANT_NAMING_DOMAINS_FQDN}}
