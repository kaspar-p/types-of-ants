# TESTS `/api/v1/tests`

## `GET /host-status`

Parameters:

```
ONE OF:
  host_id: string;
  host_name: string;
```

Get the current test status of all projects on a single host.

## `GET /host-status-history`

Parameters:

```
ONE OF:
  host_id: string;
  host_name: string;
history_length: "hour" | "day" | "week"
```

Get the test status history on a single host for the last `history_length` of
time, across all projects on that host.

## `GET /project-status`

Parameters:

```
ONE OF:
  project_id: string;
  project_name: string;
```

Get the test status of a project across all hosts, aggregated.

## `GET /project-status-history`

Parameters:

```
ONE OF:
  project_id: string;
  project_name: string;
history_length: "hour" | "day" | "week"
```

Get the test status history of a project across all hosts, aggregated. Reaches
back `history_length` into time to get the project status.

## `GET /host-project-status`

Parameters

```
ONE OF:
  project_id: string;
  project_name: string;
ONE OF:
  host_id: string;
  host_name: string;
```

Get the test status of a current project, on a host. This relies on the most
recent test data.

## `GET /host-project-status-history`

Parameters

```
ONE OF:
  project_id: string;
  project_name: string;
ONE OF:
  host_id: string;
  host_name: string;
history_length: "hour" | "day" | "week"
```

Get the status history (over the past `history_length`) of the tests for a
single project, on a single host.
