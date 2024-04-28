# ant-host-agent

The purpose of ant-host-agent is to be a daemon running on each node running one
of the projects. It's meant to be general purpose, and have a knowledge of the
projects. That is, ant-host-agent should run the same, independent of the
projects running on the same host.

This implies the invariant that every host will run ant-host-agent, even if it
isn't running any other projects. But it should also be able to run with just
ant-who-tweets, or potentially ant-who-tweets, ant-on-the-web, and even
ant-owning-artifacts.

## design

The ant-host-agent daemon will be a webserver, since that is likely going to be
the simplest way to communicate.

**Requirements:**

1. `ant-host-agent` will be a regular project.
   1. That is, the build servers will build it and vend it, just like any other
      project.
   1. The only exception is that ant-host-agent will need to come "preloaded"
      onto a machine, to do further operations
1. `ant-host-agent` will be able to collect the logs of a service upon request
   and serve them back to the requester.
   1. Perhaps bundling them together into a single `.tar` to serve back.
   1. This includes its own logs, since it is a regular project.
1. `ant-host-agent` will always respond to a `ping` with a `pong`, as a shallow
   health-check.
1. `ant-host-agent` will be able to kill projects running on that host.
1. `ant-host-agent` will be able to start new projects on that host, provided
   with a build artifact.
   1. It might require other things, like shell scripts or SQL migrations, we
      will see.
1. `ant-host-agent` will be completely stateless between requests and responses.
1. `ant-host-agent` will restart itself upon error or crash.

## api

Based on the requirements above, `ant-host-agent` will have the following API:

1. `GET|POST /ping`
1. `POST /launch_project`
   1. Launch a project on the host machine
1. `POST /get_project_logs`
   1. Get the logs of a project from the host machine
1. `POST /kill_project`
   1. Kill the project on the host machine

OR

1. `GET|POST /ping`
1. `POST /project`
   1. Launch a project on the host machine
1. `GET /project/:project-id/logs`
   1. Get the logs of a project from the host machine
1. `DELETE /project/:project-id`
   1. Kill the project on the host machine
