# `anthill`

The `anthill` project is, unlike other projects, not a long-running service. It
is the build system that all of the projects within the types-of-ants monorepo
are built with.

The purpose of `anthill` is to give CLI and programmatic hooks into the building
of a project. This is important for the build tooling, for example, for projects
like `ant-owning-artifacts`, the build and deployment server.

## design

1. The `anthill` project needs to compile any of the projects
1. The `anthill` project needs to declare a common format, similar to
   `package.json`, for what a package requires to build and be deployed.
1. The `anthill` project needs to be installable by myself, on my native
   machine, somewhere in my path.
1. Since this is the easiest project to test, the `anthill` project needs to be
   completely unit-tested for each of its features.
