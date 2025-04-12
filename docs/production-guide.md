# Production Guide

Notes I take as I attempt to deploy various services to various hosts,

## Production guide for `ant-on-the-web`

Get new changes by `cd ~/types-of-ants && git pull`.

Everything can then be done with the deployment script:

```bash
./scripts/deploy.sh ant-on-the-web
```

This will take a while, but builds the ant-on-the-web website, server, builds
them into the local `./projects/ant-on-the-web/build` directory in the correct
way, and runs `make install`, which brings those artifacts into the `~/service`
directory.

The final step that this script performs is to link the `ant-on-the-web.service`
daemon and restart the daemon, which should be all the steps.

You may run into problems, make sure that `~/types-of-ants/.env` exists and is
correct for the case. Especially be wary of the `DB_HOST` environment variable
for webservers on a machine that isn't the database machine.

## Production guide for `ant-who-tweets`

Get new changes with `cd ~/types-of-ants && git pull`. The deployment looks
exactly the same as `ant-on-the-web`, so just run:

```bash
./scripts/deploy.sh ant-who-tweets
```

And the rest is done for you. See the working directory for logs.

## Production guide for `ant-gateway`

First, make sure that this host is the one being port-forwarded to on the local
network. By going to <http://192.168.2.1> > Advanced > Port Forwarding, make
sure that the right host is pointed to.

The hosts should be named according to their `/etc/hostname` file. Last checked
it was `antworker001` running the ant-gateway.

## Production guide for `ant-data-farm`

Follow the "daemonization" guide for `ant-data-farm` to make it a systemd
service. Then just run:

```bash
sudo systemctl enable ant-data-farm.service && \
sudo systemctl start ant-data-farm.service
```

to start it. The logs can be read via:

```bash
sudo journalctl -u ant-data-farm.service
```

or even through `docker` via:

```bash
docker logs $(docker ps -q)
```

You can install migrations via:

- Log onto the host with the DB (000 currently)

```bash
cd ~/types-of-ants
git pull
cd projects/ant-data-farm/data/sql
```

- Connect via `psql -U typesofants -h 0.0.0.0 -p <port>`.
- Apply each migration file with `\i <file-name>`

## `ddclient`

I'm not paying $46 a month for a static IP address from Bell. We use a daemon
job to run in the background on one of the hosts to hit CloudFlare APIs to
update the value of an IP when we detect that it's changed. This is `ddclient`.

On one of the hosts (I've chosen `antworker002`), install a known working
version:

```bash
mkdir -p ~/installs
cd ~/installs
wget https://github.com/ddclient/ddclient/archive/refs/tags/v3.11.2.tar.gz
tar xvfa v3.11.2.tar.gz
cd ddclient-3.11.2
./autogen
./configure \
  --prefix=/usr \
  --sysconfdir=/etc/ddclient \
  --localstatedir=/var
make
make VERBOSE=1 check
sudo make install

sudo chown ant /etc/ddclient/ddclient.conf
sudo chown ant /var/cache/ddclient/ddclient.cache
```

And the `/etc/ddclient/ddclient.conf` file needs to be edited with contents.
This can be done with:

```bash
cd ~/types-of-ants

./projects/ant-gateway/ddclient/init-ddclient.sh '.env'
```

where the password field is filled in. Keep the single quotes around it!

Then, running `ddclient` will begin the process. You can check on it via the
logs with `ddclient -query`.
