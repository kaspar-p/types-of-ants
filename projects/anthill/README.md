# The anthill format

Anthill is a format for building and deploying projects of various types. It's
meant to be used on a "runnable", and there may be fewer runnables than
packages. For example, `ant-data-farm-client` is not a runnable, it's just a
standalone Rust library that builds into various other projects.

Anthill has a single goal: turn source code into a deployable artifact.

Anthill works with an `anthill.json` file at the root level of a project. The
structure is:

```json
{
  // The name of the runnable, should be globally unique. Generally the name of
  // the binary the runnable produces, or the .service systemctl unit file.
  "project": "your-project-name",

  // Unused
  "version": "1.0.0"

  // The project type dictates the type and output of the build system used.
  // See further for each project type, what it assumes about the project,
  // and how they are deployed.
  "project_type": "rust-binary" | "docker-compose" | "custom"
}
```

## `rust-binary`

The `rust-binary` project type is common and easy to deploy. The build system
will run `cargo build` (potentially with `--release`).
