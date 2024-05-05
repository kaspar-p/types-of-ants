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
