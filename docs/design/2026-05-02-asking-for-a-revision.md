# Asking for a revision

The build CLI does `UpsertRevision()` and gets `rev1`. It then performs the
build, and uses `rev1` in all of its future changes. The revision `rev1` is
marked ACTIVE.

If all builds succeed, each registers `(rev1, artifact)`. Once the revision has
collected artifacts for all relevant architectures, the revision is immutable,
new registrations to `(rev1, ...)` throw.

If one of the builds fails or the CLI crashes before artifacts for all
architectures are registered, that's fine. When the CLI starts again and
performs `UpsertRevision()`, the server hands back `rev1`! Assuming all
artifacts successfully get registered, this revision then progresses through the
pipeline.

The next time build is invoked, `UpsertRevision()` will return a newer revision,
`rev2`, since the other one has since become immutable, and this will be the
basis of a new build and deployment.

## Advantages over the push method

Previously, revisions were generated lazily, as the final step of the build.
This caused race conditions if builds for multiple architectures for the same
project both did a create-or-get operation at the same time. The build would
have to be retried.

It also caused problems when a build would fail and become abandoned. The log
looked something like:

```py
RegisterArtifact(proj, v1, x86) => generated rev1
# Missing some architectures! Something crashed, closed my laptop, etc.

... time passes ...

RegisterArtifact(proj, v2, x86) => generated rev2
RegisterArtifact(proj, v2, arm) => use rev2
RegisterArtifact(proj, v2, raspbian) => use rev2

... pipeline starts ...
```

and the previous revision rev1 gets abandoned.

## Interaction with "deployment version"

The build version seen on the host is something like:

```txt
532-2026-5-2-20-10-fa31d1c2
```

Which is intended to be globally ordered, but ultimately human-readable. This
version used to be 1:1 with the revision, where `v2` would cause the build
system to abandon the revision previously associated with `v1`.

This is no longer the case. This build version is now just a LABEL that the
build client can choose to associate with the revision. It still has to be
globally unique (to prevent on-host conflicts), but no longer influences how the
server chooses to create new revisions.

However, the build version is used to ensure that artifacts for all
architectures were built from the same content. That is, it is INVALID to deploy
a revision `rev` that points to artifacts like:

```python
(rev, proj, x86, v1)
(rev, proj, arm, v2)
(rev, proj, raspbian, v2)
```

which may happen if the `v1` client run crashed. In this case, the revision
`rev` still "wants" the x86 build. This is a best-effort mechanism to ensure
that the same repository content is used across each of the build architectures.
