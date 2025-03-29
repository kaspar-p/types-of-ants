# Production Guide

Notes I take as I attempt to deploy various services to various hosts

## Production guide for `ant-on-the-web`

Install dependencies and build the project with

```bash
cd ./projects/ant-on-the-web/website
npm ci
npm run build
```

This will take a while, and will create a `./out` directory with static HTML,
CSS, and JavaScript build artifacts. Copy these into the right location with:

```bash
mv ./out ../server/static
```

for the Rust webserver to pick them up. Build the rust webserver with:

```bash
cd ../server
cargo build
```

which will also take a long time. It should be daemonized on this host (see
[./libre-notes.md]), and we can restart it with:

```bash
sudo systemctl restart ant-on-the-web.service
```

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
