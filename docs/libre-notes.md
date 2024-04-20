# Notes taken why onboarding a new piece of types of ants hardware

> April 14, 2024

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

And SSH in with `ubuntu@<ip>`. Mine was `192.168.2.53`. Then change the password! Then, run (this will take a while):

```bash
sudo apt update
supo apt upgrade
```

And in `/etc/cloud/cloud.cfg` change `preserve_hostname: false` to `true`. Change the hostname to the one you decide on for this machine. Others are named `workerant<num>`. Set it with:

```bash
sudo hostnamectl set-hostname <name>
```

and that's it!
