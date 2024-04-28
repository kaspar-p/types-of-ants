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
  for a web server that likely has many hosts running the same binary, but for
  something like a database not that fine. This would likely be a web server
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
