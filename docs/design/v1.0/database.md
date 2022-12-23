# Database

The database will be a PostgreSQL or MySQL relational database. It will have the following databases:

- prod
- beta

The prod and beta databases will be identical in structure, just the data within them will be different.

The tables will be:

- `tests` will be a static table, each row corresponding to a test and its label.
- `test-status` will contain the data pertaining to passing/failing tests.
- `deployments` will contain the data pertaining to the deployment of projects.
- `projects` will be a static table, each row corresponding to a project and its current version, location on the machine, and more. This is like a mini version of a registry service.
- `suggestions` will contain the user-suggested suggestions.
- `users` will contain user data, since that is going to be a typesofants.org v1.0 requirement.

## `tests` (only changes when tests are added or removed)

```sql
test        varchar(50) # The unique test ID
test-label  varchar(50) # The human-readable label for the test
project     varchar(50) # The unique project ID for which project this test is testing
```

## `test-status` (added to every 5 minutes on tests running)

```sql
test        varchar(50) # The type of test that is currently running. Corresponds to a key in the 'tests' table.
test-id     varchar(50) # Randomized test id
start-date  Date        # A timestamp for the beginning of the test
end-date    Date        # A timestamp for the end of the test
status      bool        # The pass/fail status of the test
```

## `projects` (static)

```sql
project         varchar(50)   # The unique project ID
project-label   varchar(50)   # The human-readable label for the project
root-directory  varchar(256)  # The root directory of the project, where the project is run from. This is where it will delete from and replace.
semver          varchar(50)   # The semver "X.Y.Z" version of the project, currently
```

## `deployment-steps` (added only when new deployment steps are created)

```sql
step        varchar(50) # The unique step ID
step-label  varchar(50) # A human-readable label for the step, like "prod", "beta", or "build"
```

## `deployments` (added to whenever new deployments happen)

```sql
project     varchar(50) # The project ID for the project getting updated
step        varchar(50) # The unique step ID of this step that's happening
from-semver varchar(50) # The version that we are updating FROM
to-semver   varchar(50) # The version that we are updating TO
begin-date  timestamp   # The time of the beginning of the deployment step
end-date    timestamp   # The time of the end of the deployment step
status      varchar(50) # The deployment status ("SUCCESS", "IN PROGRESS", "FAILED")
```

## `suggestions`

```sql
user        varchar(50)   # The user ID of the user that submitted the suggestion
suggestion  varchar(256)  # The content of the suggestion
time        timestamp     # When the suggestion was submitted
status      varchar(50)   # The status of this suggestion: LIVE, DECLINED, DEPRECATED, UNSEEN
```

## `users`

```sql
id      varchar(50)   # The unique user ID
name    varchar(50)   # The username for the user
email   varchar(256)  # The email of the user
joined  timestamp     # The time that the user signed up
```
