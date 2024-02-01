## DEPLOYMENTS `/api/v1/deployments`

### `GET /in-progress-deployments`

Get the current projects that have in-progress deployments, and their statuses.
Includes information on which hosts those projects are in-progress on.

### `GET /project-history`

Parameters:

```
project: string
```

Get a history of deployments, describing what changed with each release. This is
like a changelog that will describe the changes to typesofants.org! This API is
parameterized by project, meaning you only get the history of a single project.

### `GET /merged-history`

```
projects: string[]
```

This API will interleave the deployment history of multiple projects together.
The history will be shown in order, for all projects given as parameters.

A single project for `projects` will is identical to `/get-history`.
