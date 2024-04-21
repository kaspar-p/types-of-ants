# Notes taken why onboarding a new piece of types of ants hardware

> April 14, 2024

## Hardware onboarding

Unbox the board, in my case I had a Libre AML-S905X-CC.

Following https://medium.com/@johnhebron/setting-up-a-le-potato-raspberry-pi-alternative-with-ubuntu-server-22-04-linux-from-scratch-8b7c22c8e4b1
download the version, I did `ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz`
unzipped it with

```bash
unxz ubuntu-22.04.3-preinstalled-server-arm64+aml-s905x-cc.img.xz
```

Using Balena Etcher (`brew install balenaetcher`) plug in the microsd card and flash it. Plug in Ethernet _before_ power, to initialize on the same wifi. Find it with:

```bash
sudo arp-scan --localnet
```

And SSH in with `ubuntu@<ip>`. Mine was `192.168.2.53`. Then change the password to any temporary ubuntu password! Then, run (this will take a while):

```bash
sudo apt update
supo apt upgrade
```

And in `/etc/cloud/cloud.cfg` change `preserve_hostname: false` to `true`. Change the hostname to the one you decide on for this machine. Others are named `antworker<num>`. Set it with:

```bash
export ANT_HOSTNAME=antworker<num>
sudo hostnamectl set-hostname $ANT_HOSTNAME
sudo cat /etc/hostname
```
and make sure it's the right output. Also `cat /etc/hosts/ and make sure there is a line like:

```txt
127.0.1.1  $ANT_HOSTNAME
```

and that's it!

## User setup, networking

Create an `ant` user on the host, and rename the `ubuntu` group to the `ants` group:

```bash
sudo adduser ant
sudo usermod -aG ubuntu ant
sudo groupmod -n ants ubuntu
```

Add the `ant` user to be able to `sudo` by adding via `visudo`:

```txt
ant  ALL=(ALL:ALL) ALL
```

To get DNS working, go to the [CloudFlare Domain](https://dash.cloudflare.com/3196bd788e22028260c62531239ac7c2/typesofants.org/dns/records) and add a record for `antworker<num>.hosts.typesofants.org` pointing to the LOCAL IP. This makes local lookups work, but doesn't expose anything.

To get the SSH key you have working to login to that user, locally from your Mac on the network, run: 

```bash
ssh-copy-id -i ~/.ssh/id_typesofants_ed25519 ant@<hostname>.hosts.typesofants.org
```

Please! Test logging into this machine with `ssh2ant <hostname_or_num>` and make sure it works before logging out again! Finally, restart the various services we have changed with, on the new host:

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

And build the project. This will take a long time, especially on these slow ass machines.
```bash
cargo build
df
```
