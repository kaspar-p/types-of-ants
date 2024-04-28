# ant-owning-artifacts

The `ant-owning-artifacts` project is a build server, and deployment
coordinator. It is meant to be a central place for projects to be compiled,
built, and packaged into known-format `.tar` files (or other format) to be
deployed onto a machine.

## design

The `ant-owning-artifacts` project will likely just be a webserver.

**Requirements:**

1. `ant-owning-artifacts` will be able to build any project on request, just to
   keep it
1. `ant-owning-artifacts` will be able to deploy any project onto any machine,
   provided that the project is not already running on that machine.
1. `ant-owning-artifacts` will place as much as it can into the artifact.
   1. This excludes things like environment variable secrets, but includes all
      source code and other build artifacts.

## api

Based on the requirements above, `ant-owning-artifacts` will have the following
API:

1. `POST /DeployProject`
   1. With the project ID and desired host that the project should be deployed
      on, will deploy a project on that host.
   1. Will not deploy the project if the host is already running that project.
1. `POST /DescribeDeployment`
   1. With the ID of a deployment, describe a single deployment, along with its
      current status.
1. `POST /ListProjectDeployments`
   1. With a project ID, will return all of the machines/hosts that are running
      that project.
1. `POST /get` with project ID, version, architecture.
   1. For a certain `architecture` (predefined options), for a project
      (predefined options), at a specific commit (or LATEST), get the artifact
      for that project to be deployed.
1. `POST /build-artifact` with project ID, version, architecture.
   1. For a certain `architecture` (predefined options), for a project
      (predefined options), at a specific commit (or LATEST), get the artifact
      for that project to be deployed.

## alternatives

1. `GET /artifact/:project-id/:commit-id/:architecture`
1. `POST /artifact/:project-id/:commit-id/:architecture`

OR

1. `GET /artifact?projectId=...+commitId=...+architecture=...`
1. `POST /artifact?projectId=...+commitId=...+architecture=...`

But to me, they aren't very clear that the work the client is doing is
_triggering_ the server to do more work. `POST /artifact` seems to me like the
client thinks it's passing all necessary data to the server, and the server is
just persisting that.
