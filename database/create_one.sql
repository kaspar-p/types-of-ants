BEGIN;

-- Create the projects table --
create table projects (
  project         serial primary key, -- The unique project ID
  label           varchar not null,   -- The human-readable label for the project
  current_semver  varchar not null    -- The semver "X.Y.Z" version of the project, currently
);

-- Create the test_types table --
create table test_types (
  test      serial primary key, -- The unique test ID
  label     varchar not null,   -- The human-readable label for the test
  project   int not null,       -- The unique project ID for which project this test is testing
  constraint fk_project foreign key (project) references projects(project)
);

-- Create the tests table --
create table tests (
  test_id       serial primary key, -- Unique test id
  test          int not null,       -- The type of test that is currently running. Corresponds to a key in the 'test-types' table.
  start_time    timestamp,          -- A timestamp for the beginning of the test
  end_time      timestamp,          -- A timestamp for the end of the test
  test_status   boolean,            -- The pass/fail status of the test
  constraint fk_test foreign key (test) references test_types(test)
);

-- Create the machines table --
create table machines (
  machine   serial primary key, -- The unique machine ID
  label     varchar not null    -- A human-readable label for the machine, like "Kaspar's Raspberry Pi"
);

-- Create the project_intances table --
create table project_instances (
  id        serial primary key,   -- A unique ID for this project instance, on this machine
  project   int not null,         -- The project that is currently running
  machine   int not null,         -- The machine the project is currently running on
  filepath  varchar not null,     -- The path to the executable of that project, currently running
  semver    varchar not null,     -- The "X.Y.Z" semantic version of the project currently running
  constraint fk_project foreign key(project) references projects(project),
  constraint fk_machine foreign key(machine) references machines(machine)
);

-- Create the deployment_steps table --
create table deployment_steps (
  step    serial primary key, -- The unique step ID
  label   varchar not null    -- A human-readable label for the step, like "prod", "beta", or "build"
);

-- Create the deployments table --
create type deployment_status as enum ('SUCCESS', 'IN PROGRESS', 'FAILED');
create table deployments (
  id                serial primary key,                 -- The unique deployment ID
  project           int not null,                       -- The project ID for the project getting updated
  step              int not null,                       -- The unique step ID of this step that happened
  from_semver       varchar not null,                   -- The version that we are updating FROM
  to_semver         varchar not null,                   -- The version that we are updating TO
  start_time        timestamp with time zone not null,  -- The time of the beginning of the deployment step
  end_time          timestamp with time zone not null,  -- The time of the end of the deployment step
  deployment_status deployment_status not null,         -- The deployment status ("SUCCESS", "IN PROGRESS", "FAILED")
  constraint fk_project foreign key (project) references projects(project),
  constraint fk_step foreign key (step) references deployment_steps(step)
);

-- Create the users table --
create table users (
  id        serial primary key,                             -- The unique user ID
  username  varchar not null,                               -- The username for the user
  email     varchar not null,                               -- The email of the user
  joined    timestamp with time zone not null default now() -- The time that the user signed up
);

-- Create the suggestions table --
create type suggestion_status as enum ('LIVE', 'DECLINED', 'DEPRECATED', 'UNSEEN');
create table suggestions (
  id                serial primary key,                               -- Unique suggestion id
  user_id           int not null,                                     -- The user ID of the user that submitted the suggestion
  suggestion        varchar not null,                                 -- The content of the suggestion
  submitted         timestamp with time zone not null default now(),  -- When the suggestion was submitted
  suggestion_status suggestion_status not null,                       -- The status of this suggestion: "LIVE", "DECLINED", "DEPRECATED", "UNSEEN"
  constraint fk_user foreign key (user_id) references users(id)
);

COMMIT;