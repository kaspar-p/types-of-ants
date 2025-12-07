# ant-zookeeper

The ant-zookeeper project is managing ants of all different flavors. It is
mostly a formalization of things that were previously done in bash scripts. The
zookeeper talks to all different types of beasts, some stable, some unstable,
some need help frequently, others can be left alone for a good while. The
zookeeper is in charge of keeping the entire zoo running.

The ant-zookeeper instance has the following PLANNED capabilities:

1. Be aware of dev/beta/prod instances. It should be the only project (so far)
   in typesofants that is aware of what "stage" its in, the others should all
   act the same no matter the stage, simply configured by their `build.cfg`
   files.

1. Owning the central secret store.

   1. This means owning the private/public TLS certificates. Which means that
      ant-zookeeper is responsible for implementing the DNS challenges involved
      in renewing those certificates.

1. Replicating secrets to the `ant-host-agent` services on hosts.

1. Rotating secrets by changing them and re-replicating.

1. Triggering deployments to hosts.

1. Triggering un-deployments, killing services.

1. Migrating persisted data from one host to another to change the host it's on.
