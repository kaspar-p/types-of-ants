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

Sometimes it doesn't work and you have to go to the network home and search for
`ubuntu-22` connected hosts there.

And SSH in with `ubuntu@<ip>`. Mine was `192.168.2.53`. Then change the password
to any temporary ubuntu password (`typesofants`).

## Terminal setup

To get cmd+left, opt+left, and other keybinds to work remotely, from the local
computer run:

```bash
infocmp -x xterm-ghostty | ssh -i ~/.ssh/id_typesofants_ed25519 ant@antworker<num>.hosts.typesofants.org -- tic -x -
```

## Tool onboarding

Then, run (this will take a while):

```bash
sudo apt update && \
sudo apt upgrade && \
sudo apt install -y net-tools dirmngr ca-certificates \
  software-properties-common apt-transport-https lsb-release curl && \
sudo apt-get install -y autoconf && \
sudo snap install jq docker btop && \
echo 'PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games:/snap/bin"' | sudo tee /etc/environment
```

Install NodeJS with NVM like:

```bash
wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
source ~/.bashrc
nvm install --lts
ln -s $(nvm which node) /home/ant/.nvm/versions/node/current
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

## Deploy deployment support server `ant-host-agent`

The ant-host-agent server is responsible for responding to requests for service
installations and deployments. However, since it's the first run, it's not
available and needs to be done manually.

```bash
./scripts/build.sh ant-host-agent dest-environment ant-host-num
```

And on the host:

```bash
$ cd ~/persist/ant-host-agent/fs/archives

$ ls
deployment.ant-host-agent.467-2025-11-28-19-51-48310c0.tar.gz

$ mkdir -p ~/service/ant-host-agent/467-2025-11-28-19-51-48310c0
$ cd ~/service/ant-host-agent/467-2025-11-28-19-51-48310c0

$ tar -xvf ~/persist/ant-host-agent/fs/archives/deployment.ant-host-agent.467-2025-11-28-19-51-48310c0.tar.gz .

$ pwd
/home/ant/service/ant-host-agent/467-2025-11-28-19-51-48310c0

$ sudo systemctl enable /home/ant/service/ant-host-agent/467-2025-11-28-19-51-48310c0/ant-host-agent.service

$ sudo systemctl status ant-host-agent.service
● ant-host-agent.service - The typesofants host agent!
  Active: active (running) since Sun 2025-11-30 02:17:18 UTC; 6s ago
  ...
```

And test it by installing itself the same script from before, from your Mac:

```bash
./scripts/build.sh ant-host-agent dest-environment ant-host-num
```
