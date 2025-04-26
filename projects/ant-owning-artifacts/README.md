# ant-owning-artifacts

The `ant-owning-artifacts` project is a build server, and deployment
coordinator. It is meant to be a central place for projects to be compiled,
built, and packaged into known-format `.tar` files (or other format) to be
deployed onto a machine.

## api

Based on the requirements above, `ant-owning-artifacts` will have the following
APIs:

1.  `POST /artifact/make`. Build a single artifact for the architecture that the
    server is.

## design

The `ant-owning-artifacts` project is a file-storing webserver.

There can either be a single `ant-owning-artifacts` for serving artifacts, or
multiple. A single instance implies that some other process must be receiving
requests for building the actual artifacts, since it's not guaranteed that the
host running `ant-owning-artifacts` will have the right architecture to build.

That is, each antworker will run an `ant-owning-artifacts` server, and they will
communicate to send files that the others might not have.

If a server receives a request for an artifact it cannot build (architecture
incompatibility), it will reach out to other servers. Assuming a fully-connected
topology, there should be at least one server that has the correct architecture.

For now, each `ant-owning-artifacts` will poll GitHub for new builds to produce,
and will build them. We won't build any of the functionality for sharing these
builds across machines.
