# Notes taken why onboarding a new piece of types of ants hardware

> April 14, 2024

## Hardware onboarding

Unbox the board, in my case I had a Libre AML-S905X-CC.

Following
<https://medium.com/@johnhebron/setting-up-a-le-potato-raspberry-pi-alternative-with-ubuntu-server-22-04-linux-from-scratch-8b7c22c8e4b1>
download the version, I did
`ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz` unzipped it with:

```bash
unxz ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz
```

Using Balena Etcher (`brew install balenaetcher`) plug in the microSD card and
flash it.

Plug in Ethernet _before_ power, to initialize on the network. Find it with:

```bash
sudo arp-scan --localnet
```

or something like

```bash
$ arp -a | grep ubuntu
ubuntu-22 (192.168.2.95) at 82:de:6c:c4:68:a9 on en0 ifscope [ethernet]
```

And SSH in with `ubuntu@<ip>`. Mine was `192.168.2.53`. Then change the password
to any temporary ubuntu password (`typesofants`)! Then, run (this will take a
while):

```bash
sudo apt update && \
sudo apt upgrade && \
sudo apt install net-tools dirmngr ca-certificates \
  software-properties-common apt-transport-https lsb-release curl && \
sudo apt-get install autoconf && \
sudo snap install jq docker btop && \
echo 'PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games:/snap/bin"' | sudo tee /etc/environment
```

Install a postgresql client matching the version `ant-data-farm` uses:

```bash
sudo apt install dirmngr ca-certificates software-properties-common apt-transport-https lsb-release curl -y
sudo sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list'
wget -qO- https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo tee /etc/apt/trusted.gpg.d/pgdg.asc &>/dev/null
sudo apt update
sudo apt install -y postgresql-client-15 postgresql-client-17
```

And in `/etc/cloud/cloud.cfg` change `preserve_hostname: false` to `true`.

Change the hostname to the one you decide on for this machine. Others are named
`antworker<num>`. Set it with:

```bash
export ANT_HOSTNAME=antworker<num>
sudo hostnamectl set-hostname $ANT_HOSTNAME
sudo cat /etc/hostname
```

and make sure it's the right output. Also add a line to `cat /etc/hosts`:

```txt
127.0.1.1  $ANT_HOSTNAME
```

and that's it!

## User setup

Create an `ant` user on the host, and rename the `ubuntu` group to the `ants`
group:

```bash
sudo adduser ant
sudo usermod -aG ubuntu ant
sudo groupmod -n ants ubuntu
```

Add the `ant` user to be able to `sudo` by adding via `visudo`:

```txt
root  ALL=(ALL:ALL) ALL
ant   ALL=(ALL:ALL) ALL
```

Then logout, you can log back in with:

```bash
ssh2ant <number>
```

## Docker installation

Make sure you can use `docker` tools:

```bash
sudo groupadd docker
sudo usermod -aG docker ant
newgrp docker
sudo systemctl restart docker
```

Alternatively, if the `systemctl restart` command doesn't work, this might:

```bash
sudo systemctl restart snap.docker.dockerd.service
```

Make sure Docker is up by getting a response from `docker ps`. You might need to
`sudo reboot`, which is safe at this point.

## Other tools installation

Install `mo`, a mustache template implementation.

```bash
mkdir -p ~/installs
mkdir -p ~/secrets
mkdir -p ~/persist

curl -sSL https://raw.githubusercontent.com/tests-always-included/mo/master/mo \
  -o ~/installs/mo
chmod +x ~/installs/mo

echo 'export PATH="$PATH:/home/ant/installs"' >> ~/.bashrc
source ~/.bashrc
```

## Networking

To get DNS working, go to the
[CloudFlare Domain](https://dash.cloudflare.com/3196bd788e22028260c62531239ac7c2/typesofants.org/dns/records)
and add a record for `antworker<num>.hosts.typesofants.org` pointing to the
LOCAL IP. This makes local lookups work, but doesn't expose anything.

To get the SSH key you have working to login to that user, locally from your Mac
on the network, run:

```bash
ssh-copy-id -i ~/.ssh/id_typesofants_ed25519 ant@<hostname>.hosts.typesofants.org
```

Please! Test logging into this machine with `ssh2ant <hostname_or_num>` and make
sure it works before logging out again! Finally, restart the various services we
have changed with, on the new host:

```bash
sudo systemctl restart sshd
```

and remove the default user, either `ubuntu` or `pi`:

```bash
sudo deluser ubuntu
sudo deluser pi
```

## Local network

### Change the hostname

Go to <http://192.168.2.1> > My Devices > Ethernet, and select the current
device. Change the hostname of the new device to `antworker<num>` in the web
terminal, if not already done so.

### Reserve the local dynamic IP

To keep this local IP reserved on the network so it doesn't change anymore, go
to <http://192.168.2.1> > My Devices > Ethernet, and select the current
antworker.

Make sure to select the IP it is on as _Reserved_.

### Open a debugging SSH port

Open a debugging port used for reading logs if not on the current network.

```txt
Name:           ssh-antworker<num>
Protocol:       Both
Internal Port:  22
External Port:  13<num>
Device:         antworker<num>
```

For example:

```txt
Name:           ssh-antworker002
Protocol:       Both
Internal Port:  22
External Port:  13002
Device:         antworker002
```

## Ghostty setup

To get cmd+left, opt+left, and other keybinds to work remotely, from the local
computer run:

```bash
infocmp -x xterm-ghostty | ssh -i ~/.ssh/id_typesofants_ed25519 ant@antworker<num>.hosts.typesofants.org -- tic -x -
```

## Setup project

First, clone the `types-of-ants` repository:

```bash
cd ~ && \
git clone https://github.com/kaspar-p/types-of-ants && \
cd types-of-ants && \
git checkout v1.0
```

Install the utilities defined in the package:

```bash
echo 'export PATH="$PATH:/home/ant/types-of-ants/bin"' >> ~/.bashrc && source ~/.bashrc
```

Install Cargo and Rust:

```bash
sudo snap install rustup --classic && \
rustup default stable
```

And build the project. This will take a long time, especially on these slow ass
machines.

```bash
cargo build
df
```

## Daemonization of `ant-host-agent`

The `ant-host-agent` project is a rust binary, and can be deployed like the
others.

On the right machine, run:

```bash
./scripts/install-rust-binary.sh ant-host-agent
```

It will return the installed version, like:

```txt
INSTALLED [ant-host-agent] VERSION [2025-04-27-12-59-43a85ad]
   when:        2025-04-27T13:00:48-04:00
   install dir: /Users/kasparpoland/service/ant-host-agent/2025-04-27-12-59-43a85ad
   version:     2025-04-27-12-59-43a85ad
   unit file:   /Users/kasparpoland/service/ant-host-agent/2025-04-27-12-59-43a85ad/ant-host-agent.service
```

And using version `2025-04-27-12-59-43a85ad`, run:

```bash
./deploy-systemd ant-host-agent 2025-04-27-12-59-43a85ad
```

## Daemonization of `ant-on-the-web`

We also need a .env file here, with the port and some database credentials. For
the file `./projects/ant-on-the-web/server/.env`, fill in the details:

```txt
DB_PG_USER=...
DB_PG_NAME=...
DB_PG_PASSWORD=...
DB_PG_PORT=7000
DB_HOST=...
```

Then, we make `ant-on-the-web` a systemd service:

```bash
sudo nano /etc/systemd/system/ant-on-the-web.service
```

with the content:

```txt
[Unit]
Description=The typesofants web server!

[Service]
Type=simple
ExecStart=/home/ant/types-of-ants/target/debug/ant-on-the-web
WorkingDirectory=/home/ant/types-of-ants/projects/ant-on-the-web/server
EnvironmentFile=/home/ant/types-of-ants/projects/ant-on-the-web/server/.env
Restart=always

[Install]
WantedBy=multi-user.target
```

And enable it with:

```bash
sudo systemctl enable ant-on-the-web.service && \
sudo systemctl start ant-on-the-web.service
```

## Daemonization of `ant-gateway`

The main thing missing is the private key certificate. I have it locally on my
laptop, so copy it to the right location _LOCALLY_ with:

```bash
scp local_path/to/key.pem \
  ant@$(anthost <NUM>):~/types-of-ants/projects/ant-gateway/secrets/ssl/beta.typesofants.org/key.pem
```

Then, we make `ant-gateway` a systemd service:

```bash
sudo nano /etc/systemd/system/ant-gateway.service
```

with the content:

```txt
[Unit]
Description=The reverse proxy for typesofants.org!

[Service]
Type=simple
ExecStart=/bin/bash -c "docker-compose -f /home/ant/types-of-ants/docker-compose.yml up --build ant-gateway"
ExecStop=/bin/bash -c "docker-compose -f /home/ant/types-of-ants/docker-compose.yml stop --build ant-gateway"

[Install]
WantedBy=multi-user.target
```

And enable it with:

```bash
sudo systemctl enable ant-gateway.service && \
sudo systemctl start ant-gateway.service
```

We can check if it's working with `docker ps` and look at the logs with
`docker logs $(docker ps -q)`.

## Daemonization of `ant-data-farm`

We make `ant-data-farm` a systemd service:

```bash
sudo nano /etc/systemd/system/ant-data-farm.service
```

with the content:

```txt
[Unit]
Description=The reverse proxy for typesofants.org!

[Service]
Type=simple
ExecStart=/bin/bash -c "docker-compose -f /home/ant/types-of-ants/docker-compose.yml up --build ant-data-farm"
ExecStop=/bin/bash -c "docker-compose -f /home/ant/types-of-ants/docker-compose.yml stop --build ant-data-farm"

[Install]
WantedBy=multi-user.target
```

And enable it with:

```bash
sudo systemctl enable ant-data-farm.service && \
sudo systemctl start ant-data-farm.service
```

We can check if it's working with `docker ps` and look at the logs with
`docker logs $(docker ps -q)`.
