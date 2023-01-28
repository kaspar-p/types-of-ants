# Database

The database will be a PostgreSQL or MySQL relational database. It will have the following databases:

- prod
- beta

The prod and beta databases will be identical in structure, just the data within them will be different.

## `test_types` (only changes when tests are added or removed)

```sql
create table test_types (
  test      serial primary key,                        -- The unique test ID
  label     varchar not null,                          -- The human-readable label for the test
  project   foreign key references projects(project),  -- The unique project ID for which project this test is testing
);
```

## `tests` (added to every 5 minutes on tests running)

```sql
create table tests (
  test_id  serial primary key                      -- Unique test id
  test     foreign key references test_types(test) -- The type of test that is currently running. Corresponds to a key in the 'test-types' table.
  start    datetime with time zone                 -- A datetime for the beginning of the test
  end      datetime with time zone                 -- A datetime for the end of the test
  status   boolean                                 -- The pass/fail status of the test
);
```

## `projects` (changed when new projects are added)

```sql
create table projects (
  project         serial primary key,  -- The unique project ID
  label           varchar not null,    -- The human-readable label for the project
  current_semver  varchar not null,    -- The semver "X.Y.Z" version of the project, currently
)
```

## `machines` (the machines currently running any software, changes made manually)

```sql
create table machines (
  machine       serial primary key, -- The unique machine ID
  label         varchar not null,   -- A human-readable label for the machine, like "Kaspar's Raspberry Pi"
)
```

## `project_instances` (the current status of the projects on each machine)

```sql
create table project_instances (
  id        serial primary key.                        -- A unique ID for this project instance, on this machine
  project   foreign key references projects(project),  -- The project that is currently running
  machine   foreign key references machines(machine),  -- The machine the project is currently running on
  filepath  varchar not null,                          -- The path to the executable of that project, currently running
  semver    varchar not null,                          -- The "X.Y.Z" semantic version of the project currently running
)
```

## `deployment_steps` (added only when new deployment steps are created)

```sql
create table deployment_steps (
  step    serial primary key  -- The unique step ID
  label   varchar not null    -- A human-readable label for the step, like "prod", "beta", or "build"
)
```

## `deployments` (added to whenever new deployments happen)

```sql
create type deployment_status as enum ('SUCCESS', 'IN PROGRESS', 'FAILED');
create table deployments (
  id          serial primary key,
  project     foreign key references projects(project),       -- The project ID for the project getting updated
  step        foreign key references deployment_steps(step),  -- The unique step ID of this step that happened
  from_semver varchar not null,                               -- The version that we are updating FROM
  to_semver   varchar not null,                               -- The version that we are updating TO
  start       datetime with time zone not null,               -- The time of the beginning of the deployment step
  end         datetime with time zone not null,               -- The time of the end of the deployment step
  status      deployment_status not null,                     -- The deployment status ("SUCCESS", "IN PROGRESS", "FAILED")
);
```

## `suggestions`

```sql
create type suggestion_status as enum ('LIVE', 'DECLINED', 'DEPRECATED', 'UNSEEN');
create table suggestions (
  id          serial primary key,               -- Unique suggestion id
  user        foreign key references users(id), -- The user ID of the user that submitted the suggestion
  suggestion  varchar not null,                 -- The content of the suggestion
  submitted   datetime with time zone not null, -- When the suggestion was submitted
  status      suggestion_status not null,       -- The status of this suggestion: "LIVE", "DECLINED", "DEPRECATED", "UNSEEN"
)
```

## `users`

```sql
create table users (
  id      serial primary key,                             -- The unique user ID
  name    varchar not null,                               -- The username for the user
  email   varchar not null,                               -- The email of the user
  joined  datetime with time zone not null default now(), -- The time that the user signed up
)
```
