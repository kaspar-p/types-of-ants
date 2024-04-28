# HOSTS `/api/v1/hosts`

## `GET /host`

Parameters

```
host: string
```

Get the information about a single host, it's shallow health status, the
projects on that host, and the versions and deployment statuses of those
projects.

## `GET /list-all`

Describes every single host currently running a typesofants.org project, which
project their running, and the version of that project their running.
