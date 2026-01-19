# Deployment Log

## Requirements

- **Artifact Suitability**: The pipeline should be able to be changed (hosts
  added to host groups, stages added at any point in the pipeline, new
  architectures added to build requirements).
- **Artifact Suitability Safety**: Pipeline changes should be _safe_. New
  code/steps/hosts should not run for the first time deep within the pipeline.
- **Batching**: Some changes need to be batched to be effective. It should be
  possible to batch something like (deploy, test) together in a way that it's
  not possible to start v2 on `deploy` while `test` is still testing v1.

## Data Structure

The "Deployment Log" will be the source of truth for all deployments. It will be
an append-only log for actions that have been completed successfully.

Each log entry at minimum, looks like:

```txt
(id, target_id, revision, event_name, created_at)
```

Meaning that for deployment `id`, the `event_name` has happened to `target_id`
at `created_at` time, with version `revision`.

## Multiple granularities

### Deploying through an entire pipeline

Deploying an entire pipeline begins with an entry like:

```sql
("deployment-id", "p-id", "version-123", "pipeline-finished-successfully", now())
```

And we can quickly lookup interesting questions about a pipeline:

- _What's the latest revision deployed through the entire pipeline?_ Go through
  and find `"pipeline-finished-successfully"` events, ordered by the revision,
  select the latest one.

- _What's the latest failed revision?_ Go through and find `"pipeline-failed"`
  events, ordered by the revision, select the latest one.

- _What are the in-progress revisions?_ Filter all events targeting that
  pipeline id that have revisions that don't have a corresponding
  `"pipeline-finished-successfully"` or `"pipeline-failed"` event.

### Deploying through a single stage

This _also_ works for multiple granularities:

- _What's the latest revision deployed through this stage?_ Go through and find
  `"stage-finished-successfully"` events targeting the stage's ID, ordered by
  the revision, select the latest one.

- _What's the latest failed revision?_ Go through and find `"stage-failed"`
  events, ordered by the revision, select the latest one.

- _What are the in-progress revisions?_ Filter all events targeting that
  pipeline id that have revisions that don't have a corresponding
  `"stage-finished-successfully"` or `"stage-failed"` event.

And it works for all other granularities.

## Step-count agnostic

The step-count and number of steps composes really nicely. Since each query just
looks for its specific markers (e.g. stages look for
`"stage-finished-successfully"`), new steps within the workflow can be added
ad-hoc into the event stream, preserving backwards compatibility.

The Deployment ID has to be generated brand-new at the beginning of the entire
pipeline, since nothing composes on _top_ of a pipeline, it's the topmost layer.
But all other layers (stage, host group, host) compose nicely.

## Build stages

This is simple to do. If a deployment has begun but the `build` stage is not
finished (see
[Deploying through a single stage](#deploying-through-a-single-stage)), then the
build is requested/waited on.

Since building is something that (at the time of writing) doesn't have a service
attached, it's really that events are emitted into the deployment log when build
artifacts are registered. For example, if a certain project only wants to deploy
onto `x86` architecture hosts, then it can emit the
`"stage-finished-successfully"` event right when that artifact is received.

Other projects may choose to emit single events for "registered an artifact" and
wait until all 3 are in the log before emitting the final step.

This allows new architectures to be introduced without complicating
previous/ongoing deployments.

## Progressing the event log

The algorithm is simple to describe. Let us define the functions:

- `next`: A function from `entry -> entry[]` defining the next set of actions to
  perform, assuming the input entry is already completed. For example, if a
  `"host-group-started"` action is the one completed, we may look up the hosts
  in the host group and return `"host-deployment"` actions _for each of them_.

- `after`: A function from `entry -> entry[]`, the "flood-filled" set of actions
  in the entire pipeline after this state. Calculated by iteratively calling
  `next`.

- `do`: A function from `entry -> ()`, perform the work of an action. For
  example, perform the `"host-deployment"` work by actually deploying to a host.

And a few data structures:

- `seen`: A set of all completed entries. For cycle detection.
- `frontier`: A list of incomplete entries `s` such that `next(a) = s` and `a`
  is complete. The "next work to do" for a pipeline.
- `batches`: A list of `(entry1, entry2)` that represent blocks of work. The
  range `(entry1, entry2)` must have atomic revisions deployed to it.

And a few terms:

- _new_: A step `a` is _new_ if all steps in `after(a)` are incomplete.
- _unlocked_: A step `a` is _unlocked_ if there are no entries like `(a, _)` in
  `batches`.
- _ready_: A step `a` is _ready_ if, given a frontier `f`, the steps in
  `after(f)` do not contain `a`.
- _doable_: A step is _doable_ if it is both _new_ and _ready_.

### Algorithm Overview

The algorithm is a scheduling loop over `revisions`, the versions of a project
currently going through the pipeline. We want to find the `frontier`, the work
that has not been completed. However, we also want to dynamically define the
next set of actions by the current action, rather than encode it at the start of
the revision or something crazy.

To find the `frontier`, we begin with the first state that all states come
after, `"pipeline-start"`. We calculate the next states, and add those to a work
queue. We need to maintain a `seen` set as well, to prevent cycles, so add to
that too.

- If each of the queued states are already complete, we calculate `next(state)`,
  add to the work queue, and progressively make our way down the pipeline.

- If the queued state is not yet complete, we add it to our `frontier` list.

Once our work queue is empty, our `frontier` list should contain all states that
can begin work.

For each of the states in the frontier, it is by-definition not complete. If the
state is not _doable_, skip it for now (see
[Appendix: Filtering for only _doable_ states](#appendix-filtering-for-only-doable-states)).
If it is, do the work! Record the fact that it's complete into the database. Add
`next(state)` into the `frontier` and `seen`, and start at the front of
`frontier` again!

#### Appendix: Filtering for only _doable_ states

States that are not _new_ mean that something changed halfway through the
pipeline. For example, if it previously went `host-deploy -> host-finished` to
`host-deploy --> host-testing --> host-finished`, the `host-testing` state would
not be _new_, since `after(host-testing)` would contain host-finished, and
that's already been completed on that host.

We need to specify `after` and not just `next` for the _new_ property, since
it's possible that an entirely new pipeline stage was added, or a new host to a
host group. This makes that "path" to a completed state longer, since we have to
go through all lower-level steps.

States that are not _ready_, may be coalescing parallel paths. Consider the case
where `host-group-finished` wants to wait for all parallel deployments in its
hosts to complete. If the first time we see that `next(host-finished)` is
`host-group-finished`, we may add this to a work queue and work on it. But it
should really have waited for all inner hosts to have completed! For that
reason, we perform the calculation on each state in the frontier. If
`host-group-finished` appears multiple times (which it would for > 1 host), we
can't yet work on it.

#### Appendix: Coalescing steps

For example, the `"pipeline-finished"` coalesces every step. Every possible path
from the root, ends up at `"pipeline-finished"`. How do we know when to write
this as complete?

We find the frontier for a pipeline by iterating `next` until we find incomplete
steps. But we aren't sure that we can do them right away because:

```txt
step1 ----> step2
       \--> step3
```

might seem like we can do step2 and step3 right now. But in fact, if we computed
the entire graph it would look like:

```txt
step1 -------------------------------------------> step2
       \--> step3 -> step4 -> step5 -> step6 --/
```

So we amend the DOABLE definition: A step `s` is DOABLE if (above is true), but
also if, for every step `t` on the frontier, `after(t)` does not contain `s`.
That is, we can only pick up that work if it will not eventually become work
again. In the above example, it would make the work order:

```txt
step1 | perform                     | add to frontier next(step1) = step3, step2
step2 | skip; step2 in after(step3) |
step3 | perform                     | add to frontier next(step3) = step4
step4 | perform                     | add to frontier next(step4) = step5
step5 | perform                     | add to frontier next(step5) = step6
step6 | perform                     | do not add step2, already there
step2 | perform, finally            |
```

#### Appendix: Cycle detection

We generally add `next(state)` into the frontier to find the edge even if they
are completed, but this means that in the presence of even simple cycles, like:

```txt
a <---> b
```

the frontier will continually add `a` and `b` while it works. So we keep an
additional data-structure, a set of seen states to prevent cycles `seen`.
Whenever we add `next(state)` to the frontier queue, we remove the ones present
in `seen`. If they aren't present, we add them to `seen`.

#### Appendix: Batching

Batching is interesting. It's the desire that `(step1, step2, step3)` happen
entirely atomically. Unfortunately with our construction all decisions are made
entirely locally. The `step1` state has no idea how many steps there are until
_anything_ happens. All it should know is that `step3` is the end of its batch.
For example, a stage knows that `stage-finished` is the end of its batch, and
when that happens, other deployments can begin in that batch.

So the amendment to the above algorithm is alongside emitting
`next(state1) -> state[]` as we go along, we also need to emit
`batch(state1) -> (state1, state2)`. This could be nothing (no batching for this
type), or `(state1, state2)` where state2 is the event that would end the batch.

Think of them like parentheses, where the start of a batch is `(` and end of a
batch is `)`. While we're flood-filling, we should be collecting the batch-end
values and building `(((`. When we encounter any `)`, we remove the matching
batch from our collection.

Once we're at the frontier, our batch collection is _the current active set of
batches across all revisions_. The batching collection therefore enforces that
we make scheduling decisions `for r in R` where `R` is sorted oldest-first, so
that older revisions take batch precedence over newer revisions.

Then, along with the _doable_ property, we filter the `frontier` we've built for
every revision (oldest first) by considering the batching collections. For every
`state` in frontier, if anything in the batching collection looks like
`(state, _)`, then there's currently a revision out for this state, and we
SHOULD NOT continue. We continue and do not consider this ready.

#### Appendix: Handles and threads

The actual work performed is usually extremely minor. For example, there are ~8
steps involved in deploying to a stage with a host group that has no hosts. The
real-world effect of this is no work, but we're still required to do all of
this.

Each time we decide to work on a _doable_ step, a separate thread or process
should be spawned to 1/ perform that work, and 2/ write its completion into the
database.

### Simple Example

Consider the simple pipeline:

```txt
[build] --> [ant-gateway/beta] -> [ant-gateway/prod]
                  antworker002          antworker001
```

where each consists of a single host (002 and 001, in this case). Similar to the
event-loop ending stages by emitting a marker for a given thing (so that the
steps within are agnostic), specific state-machine steps can be "promoters",
representing each of the `-->` arrows.

For example, if the latest deployment event for a given deployment was
`"stage-finished-successfully"` for the `[build]` stage, then the promoter step
would know to emit a `"stage-started"` _marker_ event for the
`[ant-gateway/beta]` stage (the next stage, based on the structure of the
pipeline), then exit.

The promoter could actually perform that work, but its best for large
transitions to have distinct promoters, since it's possible that there are
multiple stages after a single stage, to be performed in a tree-like structure.

That is, for _each state_, there is a decision predicate that determines whether
or not to perform that work and emit the next event. For example, take something
like `"host-deployed"`. The function in charge of `"host-deployed"` would check
for the (single) `"host-artifact-replicated"` event, given no `"host-deployed"`
event. If it sees that the host has had it's artifact replicated but no
deployment, then it knows to do that work.

```rs
if host_artifact_replicated() && !host_deployed() {
  perform_host_deployment();
  write("host_deployed");
}
```

Each stage would look similarly. In places where tree-like structure is desired,
like branching to other stages, small promoter steps should be injected that
write _multiple entries into the log_ based on a single event (fan out).

```txt
          /--> [stage1] --\
         /                 \
[build]-|                   |--> [stage3]
         \                 /
          \--> [stage2] --/
```

```rs
if build_finished() {
  write("stage-started", stage1);
  write("stage-started", stage2);
}
```

In the case of consolidation, promoter steps should be injected that listen for
multiple events before writing a single event.

```rs
if stage_finished(stage1) && stage_finished(stage2) {
  write("stage-started", stage3)
}
```

In the case of dynamically-changing pipelines (if APIs can change them), this
all should be done via lookups on the structure of the pipeline. So really the
handler for `"stage-finished"` has a lot of work to do, since it has to
understand if it's the only stage in the same parallel structure in which case
it should `write("next-stage")`. If it's just one in a parallel branch, in which
case it should lookup the status of the other parallel branches and promote
based on all of their criteria.

## Gating events

For example, we might want the invariant that the "ant-data-farm/beta" and
"ant-host-agent/beta" should be deployed before _either_ go to prod, that is,
the deployment looks something like:

```txt
          /--> [ant-data-farm/beta]  --\
         /                              \
[build]-|                                |--> [ant-data-farm/prod] --> [ant-on-the-web/prod]
         \                              /
          \--> [ant-on-the-web/beta] --/
```

This is simple to do.

If a deployment's latest event is that it's finished the `build` stage (again,
see [Deploying through a single stage](#deploying-through-a-single-stage)), then
the workflow knows (based on the structure of this particular pipeline), to emit
two events relating to stages beginning their workflows, see
[Progressing the event log](#progressing-the-event-log).

## Introducing new workflow steps

Say that we didn't previously have post-deployment testing, and we're
introducing it now. Consider the simple pipeline:

```txt
[build] --> [ant-gateway/beta] -> [ant-gateway/prod]
                  antworker002          antworker001
```

That is, the event log looked (before adding testing) like:

```txt
pipeline-start                              (p1)
  stage-started                             ([build])
    build-stage-received-x86-artifact
    build-stage-received-aarch64-artifact
    build-stage-received-raspbian-artifact
  stage-finished-successfully               ([build])

  stage-started                             ([ant-gateway/beta])
    host-group-started                      (hg:ant-gateway/beta)
      host-started                          (antworker002)
        host-artifact-replicated
        host-deployed
      host-finished-successfully            (antworker002)
    host-group-finished-successfully        (hg:ant-gateway/beta)
  stage-finished-successfully               ([ant-gateway/beta])

  stage-started                             ([ant-gateway/prod])
    host-group-started                      (hg:ant-gateway/prod)
      host-started                          (antworker001)
        host-artifact-replicated
        host-deployed
      host-finished-successfully            (antworker001)
    host-group-finished-successfully        (hg:ant-gateway/prod)
  stage-finished-successfully               ([ant-gateway/prod])

pipeline-finished-successfully              (p1)
```

We want to introduce, right after the `host-deployed` step, a new `host-tested`
stage. The changes to do this, are simple: the state that looking to write
`host-finished-successfully` based on the final event being `host-deployed`
changes to now looking for `host-deployment-tested`.

However, doing this and restarting the service would inject the new "testing"
requirement halfway through deployments. For example, new steps (new code!)
could happen for prod but not beta, if the step was created halfway through.

For this reason, we need to correlate the "timestamp" that a new step was
introduced to be sure only revisions AFTER the introduction of that step include
the new step. Note that it's fine to include the new step if the revision is
"too early", and has never seen the next.

For `"host-deployment-tested"`, we can apply the step only if the revision has
never seen a `"host-finished-successfully"`, the step after
`"host-deployment-tested"`.

That is, new steps unfortunately require a small database migration.

```sql
insert into deployment_step_introduction
  (deployment_step_name, revision_after)
values
  ('host-deployment-tested', <latest revision registered>)
```

and each step should ignore steps for project revisions that are _before_ their
own introduction.

### The `next()` function

For each deployment step, we need to understand what is next. This has to be
encoded in code, since we want to be able to change this at any time.

For linear, non-repeating steps, a simple `String -> String` map is sufficient.
For example, imagine:

```txt
"pipeline-started" -> "pipeline-deploying"
"pipeline-deploying" -> "pipeline-finished"
```

with each state being atomic. However, with the concept of "stages" and
composable blocks of a deployment, we'd expect something like the above diagram
(see [Introducing new workflow steps](#introducing-new-workflow-steps)). For
example, we want to complete the `"host-deployed"` step for each host.

So really, the uniqueness is `(deployment_id, target_id, event_name)`. Some
examples:

```rs
// Hosts are finished after they are deployed.
next("d", "host-1", "host-deployed") // [("d", "host-1", "host-finished")]

// Host groups are finished after all hosts in that group are finished
next("d", "host-1", "host-finished") // [("d", "hg-A", "host-group-finished")]
```

This sort of logic necessitates being able to run arbitrary code to determine
the next state, since the "host group of a host" is not encapsulated in the
deployment event log.

And note the "vector of next states" as the return value, this allows pipelines
to "fan out" into many sub-deployments in parallel.

## The edges3() function

Assume that we have an algorithm `edges3()` to find triples of "edges" of the
deployment. Basically "where work needs to happen" for the pipeline. It returns
the first (in `next()` order) event-triplets `(x, y, z)` such that:

1. `next(x) == y`
1. `next(y) == z`
1. `x` has been completed
1. `y` has not been completed.

```py
e1 is complete
e2 is complete
e3 is incomplete
e4 is incomplete

edges3() # (e2, e3, e4)
```

During the introduction of a new step `e_new`, we'd find many deployments
`(e1, e_new)` where e1 is complete. We have the following cases then for
`(e1, e_new, e2)`, where `next(e1) == e_new` and `next(e_new) == e2`:

`010: (e1=complete, e_new=incomplete, e2=complete)`: This means that this was a
new step introduced, and previously in the pipeline. We should do nothing, since
the new work to do is deeper in the pipeline.

`011: (e1=complete, e_new=incomplete, e2=incomplete)`: This means that a new
step was introduced, but late in the pipeline. We _may_ need to perform e_new.

- If the introduction revision of `e_new` is AFTER the current ongoing revision
  we're progressing, we should do `e_new`. This step was introduced, but could
  have been introduced months ago.
- If the introduction revision of `e_new` is BEFORE the current ongoing
  revision, but the revision has never had an `e2` completed, it means that the
  new event was introduced during the pipeline's progression, but BEFORE it
  would have been skipped. In this case, we should do `e_new`.
  - If we didn't do `e_new` in this case, we'd need to wait for entirely new
    artifacts to be produced to take advantage of new deployment steps.
  - This is an optimization, but feels important.
- If the introduction revision of `e_new` is BEFORE the ongoing revision, and
  the revision has had a completed `e2` anywhere, it means that we're
  introducing this step halfway through a pipeline, in a bad way.
  - If we did `e_new` at this moment, it's like deploying to beta without tests,
    then tests were introduced, then deploying to prod with tests. New code
    needs to do it all!

And the following cases are impossible, we should assert() that these never
happen:

```bash
000: (e1=complete, e_new=complete, e2=complete)   # edges3() should never return complete in index=1
001: (e1=complete, e_new=complete, e2=incomplete) # edges3() should never return complete in index=1
010:
011:
100: (e1=incomplete, e_new=complete, e2=complete)     # edges3() should never return incomplete in index=0
101: (e1=incomplete, e_new=complete, e2=incomplete)   # edges3() should never return incomplete in index=0
110: (e1=incomplete, e_new=incomplete, e2=complete)   # edges3() should never return incomplete in index=0
111: (e1=incomplete, e_new=incomplete, e2=incomplete) # edges3() should never return incomplete in index=0
```

```rs
let (completed_step, todo_step, next_step) = edges3(next_map);

match (completed_step, todo_step, todo_step) {
  // 001
  (Complete(e1), Complete(e_new), Incomplete(e2)) => {
    perform(e_new);
    write_completed(e_new);
  }
  (Complete(e1), Complete(e_new), Incomplete(e2)) => {
    perform(e_new);
    write_completed(e_new);
  }
}
```

Steps that are old _don't change_. The code immediately starts knowing
`next("host-deployed")` is `"host-deployment-tested"`, which gets skipped if
appropriate.
