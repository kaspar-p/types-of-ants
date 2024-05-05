# Developer Guide

The developer guide for typesofants. Should always be kept up to date!

## Architecture

One of the tenets of typesofants is that it should be entirely without runtime
dependencies. That is, no use of live services like online databases like
Firebase or DynamoDB, no runtime authentication provider, nothing. I want
typesofants to not _need_ to evolve with the web, and require minimal upkeep.

That tenet is broken in DNS hosting/providing. There is only so much that can be
done solo, and domain registration and "IP pointing" in general is hard to do
alone.

Currently we use CloudFlare, but in the past we used Google Domains. Only Kaspar
has the credentials for the site but it's likely not needed for anyone else to
access it.

### Components

There are the following components:

- `ant-data-farm`: The database, a PostgreSQL database running out of a Docker
  container with mounted volumes to maintain persistent state.
- `ant-on-the-web`: has two parts, the website, and the webserver. The website
  "doesn't exist" at runtime, just like source code doesn't exist at runtime.
  It's statically compiled into a bunch of JavaScript, CSS, and HTML as a part
  of the build process, and those files are served by the webserver.
- `ant-gateway`: The reverse proxy. Only a single machine can be responsible for
  answering requests directed towards a domain, and then fan-out to the
  individual webservers from there. It's possible that this is the same machine
  as the webserver itself. It is an NGINX webserver running in a docker
  container. Certificates are always a pain, I've downloaded the secret ones for
  `beta.typesofants.org` and those are being used.
- `ant-host-agent`: is another webserver, a binary that runs on each host.
  Usually bound to port 4499, has a single usable route today `/ping` that
  returns the string `healthy ant`. It's used for an extremely basic monitoring
  system.
- `ant-who-tweets`: a CRON job that runs on one of the hosts and tweets every 24
  hours at 6PM MST. Tweets one of the many released ants at random, people love
  it!
- `ant-just-checking-in`: One of the requirements to running a web service is
  knowing if it's working. Putting code up and just hoping that things are still
  working is not a good strategy. Second-worst is letting users complaints be
  the notification. Has the capability of pinging websites and checking for
  `200 OK` response codes, as well as hitting other ant hosts and checking for
  `healthy ant`. Does not put this data anywhere, that's a WIP.

The following components are more experimental, may never see the light of day:

- `ant-building-projects`: The build servers, for eventual CI/CD deployments.
  Making changes currently involves _taking down_ the process and then rebooting
  it, or at least restarting the daemon that starts that process. This is fine
  for a webserver that likely has many hosts running the same binary, but for
  something like a database not that fine. This would likely be a webserver
  listening on a host, and would checkout, build, and store build artifacts.
  Those artifacts would be stored until the next time a deployment is needed.
- `ant-owning-artifacts`: IDK. Probably the same as the previous one, but
  thought that the server building might be better at building with less
  storage, and a different fleet would own the artifact.
- `ant-metadata`: A metadata store about all of the hosts in the fleet, the
  projects each host is running, the recent/ongoing deployments to those hosts,
  and more.
- `ant-using-email`: not a project, just a name for a Typescript script for
  sending automated emails, used for the NYE 2023 email.
- `anthill`: A declarative build system, so that `ant-building-projects` would
  know how to take a git repository and turn it into build artifacts.

## Hosts

Since typesofants is meant to be self-reliant, everything is self-hosted. They
are physical machines nailed to Kaspar's wall.

There is a `bin` directory where nice scripts go, that should be added to a
developers `PATH` variable. For example, `ssh2ant` just takes a host number and
performs `ssh` to the right user, using the local PEM file. Like `ssh2ant 000`
will attempt to connect to `antworker000.hosts.typesofants.org`. That domain is
aliased to a local IP, `192.168.something`, so it's local to the network.

Try it! If you run `curl antworker000.hosts.typesofants.org:4499/ping` you
should get the `healthy ant` response back.

## Developing on `ant-on-the-web`

Probably the more complicated one. The website can be run directly, locally, to
speedup changes.

Running `npm run dev` in the package will start the webserver on
`localhost:3000`. It by default will attempt to connect to the webserver that
runs the API, which is `ant-on-the-web/server` on `localhost:3499`, so run that
with `cargo run` in a terminal tab.

The communication flow is:

```txt
website <-> backend API <-> database
```

as any three-tiered web service is built, these days. Any changes made to the
frontend will immediately take effect!

## Developing on `ant-data-farm`

Start from top-level with `docker-compose up -d ant-data-farm` and it will start
fine. You might be missing some `.env` variables for all of this
connection/authentication BTW, ask Kaspar about them.

## Mock feature development

For example, a hypothetical new "login" feature would be built in the following
way:

First, we make frontend changes to the site, adding new pages/components
required to signup and log the user in. These will be seen in the website when
running `npm run dev`. This code is going to all by in
`ant-on-the-web/website/src`. The buttons can do nothing, they can throw
exceptions, whatever, but things just need to look nice.

Then, make API changes, likely for new routes like `/api/signup` and
`/api/login`. That code will be in `ant-on-the-web/server/src`. Implementing the
state-machine that is user authentication is a bit complicated, but that's ok.
Not everything has to be easy.

Finally, we make database changes. If the database already has the tables/schema
that we need, then no changes in the schema, but likely the DB client needs to
change. We write our own client, meaning it's a semantic object with methods
like `get_all_ants()` rather than just making network calls. Of course, it makes
network calls under the hood, but the methods are nice.

A service providing nice clients or a "mini-SDK" for itself is one of my
favorite things about working at a large web services company: everything has a
standardized client able to be generated in all major languages! Typesofants
isn't quite at the "all major languages" thing yet, but that's why everything is
written in the same language :).

Rust, BTW. Everything's written in Rust. You'll need to install that and NPM to
work on typesofants. I'm sure you can figure it out. Rust is idiomatic and has
no external "manager", but for Typescript I use the `nvm` tool.

Anyway, database changes. The schema changes are applied via "migrations", which
you can see in the `ant-data-farm` directory. That's where you would go to find
out if the schema you need is already there, by the way. Migrations are applied
in lexicographical order, the same order files are displayed in directories.
That's why the are prefixed with numbers, to apply in the right order.
Migrations should be written as a transaction.

After we have determined if the database supports our use-case or if we need
schema changes, we have to restart the database locally and apply those
migrations. New migrations need to be added to the `Dockerfile` of the database,
to be copied into the image and subsequently applied.
