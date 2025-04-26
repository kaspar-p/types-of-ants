# ant-naming-domains

Owns the `ddclient` instance on one of the hosts that is responsible for
updating CloudFlare IP mappings when the public IP of the physical location that
typesofants is deployed changes.

## Installing ddclient on a host for the first time

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

## Running ddclient

The `/etc/ddclient/ddclient.conf` file needs to be edited with contents. This
can be done with:

```bash
cd ~/types-of-ants

./projects/ant-gateway/ddclient/init-ddclient.sh '.env'
```

where the password field is filled in. Keep the single quotes around it!

Then, running `ddclient` will begin the process. You can check on it via the
logs with `ddclient -query`.
