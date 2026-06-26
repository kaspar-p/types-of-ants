# Pipelines

## Terminology

- Revision: An atomic set of changes, maps to a set of _artifacts_ that are built software. One revision corresponds to a single _pipeline_. Revisions are tied to human-readable versions that are set by the build system. Revisions are "activated" and immutable when all required artifacts are registered, and are otherwise mutable and continue to take updates. So they receive a stream of changes to artifacts until the condition is met, then locked, and begins to propagate. The next artifact update will instead create a new revision.

- Pipeline: A Directed Acyclic Graph (DAG) of "task nodes" that combine to complete a deployment for a given project. Are _always_ project specific, there are no cross-project pipelines. New ones are generated when a new revision is activated.

- Node: A single task in the pipeline. This could be as small as "log that we're here" if the implementor decides that, or as large as "apply Terraform" if they want. The content of the node is entirely independent of the actual task being performed.

## Concurrency

Multiple pipelines can be active for the same project, for example a `rev-1` could be finishing a production deployment while a `rev2` (the next one) just begins the pre-production artifact packaging + replication.

Concurrency dangers are avoided by marking tasks as `mutates: "thing"` where nodes that all `mutates` the same `"thing"` queue in revision-sequence-order. That is, a pipeline like:

```txt
[start] -> [m1, mutates: "thing"] -> [end]
```

is gauranteed to run the `[m1]` node for a revision `rev1` before a `rev2`. It is NOT guaranteed that the `[end]` node in any order, however.

Multiple nodes that all declare a `mutates: "thing"` on the same `"thing"` are sequenced together, meaning on a pipeline structured:

```txt
[start] -> [m1, mutates: "thing"] -> [m2, mutates: "thing"] -> [end]
```

it's guaranteed that BOTH `[m1]` and `[m2]` will complete for `rev1` before even `[m1]` starts for `rev2`. That is, the earliest usage of a `mutates` blocks future revisions. This keeps things sane.

Note that the `mutates: String` is just opaque, and any differences in string equality will result in concurrent deployments of that node!

## Dispatch

Each node calls a central `dispatch` with the given "event" data, basically a pile of data from the initial DAG creation that is handed back to `dispatch` when the node is activated.

For example, for a `DeployToHost` node you might want to stuff `event` (as a serialized JSON or any structure you feel like), with something like:

```json
{
    "hostname": "my-host.deployment-pool.example.com",
    "artifact_path": "/artifact/v1.tar.gz"
}
```

Or whatever your host-deployment layer needs to complete that request.
