# Deployment System

## Background

...

## Requirements

### Concurrent deployments

Multiple revisions can be in-flight simultaneously. Deploying revision `rev1` to prod and
revision `rev2` to beta must not block each other. Serialization happens at the resource
level, not the pipeline level.

### FIFO-per-resource scheduling

For a given physical resource (host+service, database, global config object/file), revisions
pass through in order. All of the `rev1` revision's work on a resource completes before any of
the `rev2` revision's work on that resource begins. No explicit locks.

### Per-revision DAGs generated on the fly

Each revision gets its own deployment DAG at activation time, based on the
current state of anthill.json and host configuration. No static pipeline
templates. No `init_pipelines`. Two revisions may have entirely different DAG
shapes (one has a migration step, the other doesn't). Bad deployments cannot
poison future ones.

### Single-project builds

Each project builds and deploys independently. No "build the world" mechanism, so a change
to `ant-on-the-web` does not require rebuilding `ant-gateway`.

### Failure blocks, not retries

A failed node is terminal. It blocks the pipeline and downstream nodes until
an operator either retries the node (manual) or cancels the pipeline. No
automatic retry. No silent infinite loops.

### Cancellation unblocks newer revisions

Cancelling a pipeline marks all its non-finished nodes as cancelled. The FIFO
scheduler treats cancelled nodes as done, unblocking newer revisions waiting on
the same resources.

### Dispatch is pure work

The `tick()` caller provides a `dispatch` function that takes a node and returns
`Result<(), Error>`. The engine handles claiming, job creation, spawning,
tracing, heartbeats, success/failure recording, and promotion. Dispatch has no
knowledge of engine internals.

### Jobs as audit trail

Every execution of a node creates a job row. Jobs record start time, heartbeats,
completion, and errors. Multiple jobs can exist for the same node (failed
attempts followed by a retry). The job table is the history of what happened.

### Heartbeat-based liveness detection

Spawned tasks heartbeat every 10s. If a heartbeat goes stale (>60s), the node
is released back to `executable` and the job is marked failed. The next tick
re-dispatches automatically. Handles server crashes and laptop-lid-closes
without manual intervention. Dispatch functions must be idempotent — re-running
after a crash should check actual state rather than blindly retry.

### Fire-and-forget tick

`tick()` claims nodes, creates jobs, and spawns tasks, then returns a
`TickHandle`. Production callers drop the handle (tasks run in background).
Test callers call `handle.join()` for deterministic completion. Nodes are
`in_progress` in the DB immediately when `tick()` returns — subsequent ticks
will not re-dispatch them.

### Long-running dispatch

Dispatch functions may run for minutes (health verification, artifact pull
polling). The engine does not block on dispatch completion. Heartbeats keep
long-running tasks alive. The engine only cares about the final
succeed/fail outcome.

### Concurrent tick safety

Multiple `tick()` calls may run simultaneously (the API is public). Safety is
guaranteed by: `FOR UPDATE SKIP LOCKED` (no double-claim), `in_progress` state
in DB (no re-dispatch), and runtime assertions (catch invariant violations).
No assumption of single-caller semantics.
