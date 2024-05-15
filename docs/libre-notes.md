# Notes taken why onboarding a new piece of types of ants hardware

> April 14, 2024

## Hardware onboarding

Unbox the board, in my case I had a Libre AML-S905X-CC.

Following
<https://medium.com/@johnhebron/setting-up-a-le-potato-raspberry-pi-alternative-with-ubuntu-server-22-04-linux-from-scratch-8b7c22c8e4b1>
download the version, I did
`ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz` unzipped it with

```bash
unxz ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz
```

Using Balena Etcher (`brew install balenaetcher`) plug in the microSD card and
flash it. Plug in Ethernet _before_ power, to initialize on the same wifi. Find
it with:

```bash
sudo arp-scan --localnet
```

And SSH in with `ubuntu@<ip>`. Mine was `192.168.2.53`. Then change the password
to any temporary ubuntu password! Then, run (this will take a while):

```bash
sudo apt update
sudo apt upgrade
sudo apt install net-tools
sudo apt-get install autoconf
sudo snap install jq docker btop

curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
[ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"
nvm install --lts

source ~/.bashrc
```

And in `/etc/cloud/cloud.cfg` change `preserve_hostname: false` to `true`.
Change the hostname to the one you decide on for this machine. Others are named
`antworker<num>`. Set it with:

```bash
export ANT_HOSTNAME=antworker<num>
sudo hostnamectl set-hostname $ANT_HOSTNAME
sudo cat /etc/hostname
```

and make sure it's the right output. Also `cat /etc/hosts/ and make sure there
is a line like:

```txt
127.0.1.1  $ANT_HOSTNAME
```

and that's it!

## User setup, networking

Create an `ant` user on the host, and rename the `ubuntu` group to the `ants`
group:

```bash
sudo adduser ant
sudo usermod -aG ubuntu ant
sudo groupmod -n ants ubuntu
```

Make sure you can use `docker` tools:

```bash
sudo groupadd docker
sudo usermod -aG docker ant
newgrp docker
sudo systemctl restart docker
```

Add the `ant` user to be able to `sudo` by adding via `visudo`:

```txt
ant  ALL=(ALL:ALL) ALL
```

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

## Reserve the local dynamic IP

To keep this local IP reserved on the network so it doesn't change anymore, go
to <http://192.168.2.1> > My Devices > Ethernet, and select the current
antworker.

Make sure to select the IP it is on as _Reserved_.

## Daemonization of `ant-host-agent`

First, we setup ant-host-agent with a .env file:

```bash
echo 'HOST_AGENT_PORT=4499' > ~/types-of-ants/projects/ant-host-agent/.env
```

Then, we make ant-host-agent a systemd service:

```bash
sudo nano /etc/systemd/system/ant-host-agent.service
```

with the content:

```txt
[Unit]
Description=Start the on-host ant manager!

[Service]
Type=simple
ExecStart=/home/ant/types-of-ants/target/debug/ant-host-agent
WorkingDirectory=/home/ant/types-of-ants/projects/ant-host-agent
Restart=always

[Install]
WantedBy=multi-user.target
```

and enable it with:

```bash
sudo systemctl enable ant-host-agent.service
sudo systemctl start ant-host-agent.service
```

## Daemonization of `ant-on-the-web`

We also need a .env file here, with the port and some database credentials. For
the file `./projects/ant-on-the-web/.env`, fill in the details:

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
