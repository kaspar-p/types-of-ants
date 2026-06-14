# The typesofants platform

There are a set of things that typesofants services can assume about their
runtime environment.

## The build system

All services are built via `anthill` (`ah`), namely `ah build <service>`. This
must correspond to a `projects/<service>` directory. The
`projects/<service>/anthill.json` is the configuration file. The build system
interface is mainly `make`, where all files and directories that end up in the
`$BUILD_OUTPUT_DIR` will be shipped to the production host.

That is, if `./srv` is meant to have a `./srv-config.toml` next to it, the final
directory structure after the Makefile is meant to have built is:

```fs
$(BUILD_OUTPUT_DIR)/
  srv-config.toml
  srv
```

The deployment system _WILL_ put other files into the final installation
directory, ones that are relevant for deployment monitoring, or features like
secrets.

## Secrets

Build-time has no understanding of secrets, a service requiring secrets will
_declare_ them in the `anthill.json` `"secrets"` key. Each name listed there
must be a unique secret identifier that the deployment service already has
registered.

Secrets will end up in the `$(installation-dir)/secrets/$(secret_name)`
directory on the production host. The `ant_library` has methods for reading text
and binary secrets back out.

Secrets can be declared `host_specific`, which means that the secret deployed
will _change_ based on each destination host. This raises the security
properties of each secret, but is annoying to set up.

## Development Runtime

Each service can be begun with `ah dev <service> [...additional args]`, where
the service does preliminary setup and then will invoke the
`./projects/<service>/.anthill/dev.sh` file. It is recommended that this file
calls `exec` on the `run.sh` file, see [Production Runtime](#production-runtime)
for more.

Most environment variables are the same, just the secrets are in the Git
repository root `secrets/dev/` directory, so should be added there.

## Production Runtime

The production runtime is `systemd` on Linux. It _requires_ that there be a
`./projects/<service>/.anthill/run.sh` file as the entrypoint, that's what
systemd will point at to run, with no arguments.

Services that wrap external software and pass command-line arguments should do
so within `run.sh`.

The environment variables are centrally defined within the deployment service
(`ant-zookeeper`) and will automatically be available during production and
development runtimes.

## Networking

All services run on a flat network, internally visible to each other, so
sensitive access must be gated by each application performing auth via a
database or statically-shared secrets files.

Typesofants runs a Consul cluster `ant-matchmaker`, which has an agent available
at each host. The `ant_library::sd::reader::ServiceDiscovery` can be used to
dynamically query this service registry for the (IP, Port) pairs for other
services.

For example, if a webservice like `ant-on-the-web` wants to reach its database
`ant-data-farm`, it can query `ant-matchmaker` for the nodes at which
`ant-data-farm` is deployed. The on-host deployment agents handle registration
and de-registration. The `ant-on-the-web` will receive the (IP, Port) pair for
that host, and connect with its credentials.
