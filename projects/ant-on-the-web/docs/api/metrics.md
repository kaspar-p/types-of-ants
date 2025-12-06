# METRICS `/metrics`

## `GET /page-access-history`

Parameters:

```
path: string
history_length: "hour" | "day" | "week"
```

For the `path`, get the anonymized number of accesses over the last
`history_length`.

## `POST /access-page`

Parameters:

```
path: string
userId: string
```

When a user visits a page, a request for this is sent entered.

## `POST /expand-ant`

Parameters

```
antId: string
userId: string
```

When the user expands to know more about an ant, a request with this type is
sent. This is used for tracking the top ants of that year, the ants that users
are interacting with the most.
