# ant-building-projects

The software that builds projects and deploys it onto machines.

## Requirements

`ant-building-projects` needs to be able to know if a project has changed. This can either be a push or pull method. It then needs to `git pull` the new changes, rebuild the project, stop the current program, and restart the program.

Building the project will be handled by the build system, like Bazel. Everything else is straightforward.

Deployment data will also need to be logged, to show on /deployments. Each step of the deployment will need to emit that data. The data includes:

- Timestamp of the start of the step
- Timestamp of the end of the step
- The version that it is upgrading from
- The version it is upgrading to
- The machine that it is deploying onto
- The project that is currently being deployed

## Architecture

### Option 1 (most complicated)

There will be three servers. One is a deployment server, really an orchestrator. It is constantly listening to new versions of projects. When it hears that project X has a new version published (in git), it tells the build server. The build server then puts "build project X" into its queue. 

When that event gets done and the binary is built for project X, the build server tells the deployment server it is finished. The deployment server then tells the third type of server, the replacement servers. They are lightweight webservers running on each machine, just listening for a "hey, there is a new binary for project X you should use" message from the deployment server. Once that message is received, they stop the project X process, and start another.

#### Deployment server

The responsibilities of the deployment server include:
- Listen for new versions in any of the registered projects
- Tell the build server that project X has a new version
- Once the build server is finished building project X, give some fraction of the machines running project X 

#### Build server

- Listen for a "start building project X" message from the deployment server
- Start building project X
- Once it is finished building (or had an error), tell the deployment server

#### Children servers

The responsibilities of the children servers include:
- Listen for the deployment server to tell it to download a new binary
- Download that binary
- Find the previous process that that project corresponds to (from the database)
- Let the process know that it is about to die (its SIGKILL handler), let it gracefully shut down
- Stop the process and start a new process using that new binary.

### Option 2 (medium complicated)

There are two servers. One is a deployment server and build server together, but there are still separate servers for the machines running the projects. That is, this option combines the deployment server and build server into a single process.

### Option 3 (least complicated)

There is a single process. On every machine running software, there is a single process that handles listening for versionings, building, and replacing of processes. 

That is, the child is really just a web server listening for calls from the central builder server.

### Decision

For now, the responsibilities will all be bundled into a single process. This means that every single machine will be responsible for everything. This is wasteful for many machines, but fine for just a few. That is, the least complicated option will be taken.

## Details

Since the least complicated option is taken, there is no need to a web server, since there are no incoming messages to any part of the process. There are outgoing messages, however. The database will be utilized extensively, both for reading and writing.
