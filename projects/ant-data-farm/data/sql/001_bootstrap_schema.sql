BEGIN;

drop schema if exists typesofants cascade;
create schema if not exists typesofants;
set SEARCH_PATH to typesofants;

drop domain if exists semver cascade;
create domain semver as varchar(50);

drop table if exists project cascade;
create table project (
  project_id uuid unique primary key default gen_random_uuid(), -- The unique project ID
  project_label varchar(255) not null, -- The human-readable label for the project
  project_live_semver semver not null -- The semver 'X.Y.Z' version of the project, currently
);

insert into project (project_label, project_live_semver) 
values 
  ('ant-on-the-web', '1.0.0'),
  ('ant-just-checking-in', '1.0.0'),
  ('ant-data-farm', '1.0.0')
;

drop table if exists test cascade;
create table test (
  test_id uuid unique primary key default gen_random_uuid(), -- The unique test ID
  test_label varchar(255) not null, -- The human-readable label for the test
  test_project_id uuid not null, -- The unique project ID for which project this test is testing
  constraint fk_project foreign key (test_project_id) references project(project_id)
);

insert into test (test_label, test_project_id)
values
  ('Test the web server for shallow healthy pings', (select project_id from project where project_label = 'ant-on-the-web'))
;

drop table if exists test_instance cascade;
create table test_instance (
  test_instance_id uuid unique primary key default gen_random_uuid(), -- Unique test id
  test_instance_test_id uuid not null, -- The type of test that is currently running. Corresponds to a key in the 'test_type' table.
  test_instance_start_time timestamp not null, -- A timestamp for the beginning of the test
  test_instance_end_time timestamp not null, -- A timestamp for the end of the test
  test_instance_status boolean not null,   -- The pass/fail status of the test
  constraint fk_test foreign key (test_instance_test_id) references test(test_id)
);

drop table if exists host cascade;
create table host (
  host_id uuid unique primary key default gen_random_uuid(), -- The unique machine ID
  host_label varchar(255) not null, -- A human-readable label for the machine, like 'Kaspar's Raspberry Pi'
  host_location varchar(255) not null -- A human-readable location, for fun
);

insert into host (host_label, host_location)
values
  ('Kaspar''s Raspberry Pi', 'Kaspar''s house')
;

drop table if exists project_instance cascade;
create table project_instance (
  project_instance_id uuid unique primary key default gen_random_uuid(), -- A unique ID for this project instance, on this machine
  project_instance_project_id uuid not null, -- The project that is currently running
  project_instance_machine_id uuid not null, -- The machine the project is currently running on
  project_instance_filepath varchar(255) not null, -- The path to the executable of that project, currently running
  project_instance_semver semver not null, -- The 'X.Y.Z' semantic version of the project currently running
  constraint fk_project foreign key(project_instance_id) references project(project_id),
  constraint fk_machine foreign key(project_instance_machine_id) references host(host_id)
);

drop table if exists deployment_step cascade;
create table deployment_step (
  deployment_step_id uuid unique primary key default gen_random_uuid(), -- The unique step ID
  deployment_step_label varchar(255) not null -- A human-readable label for the step, like 'prod', 'beta', or 'build'
);

insert into deployment_step (deployment_step_label)
values
  ('build'),
  ('beta'),
  ('prod')
;

drop table if exists deployment_sequence;
create table deployment_sequence (
  deployment_sequence_from uuid not null, -- The starting state of the deployment step[]
  deployment_sequence_to uuid not null, -- The ending state of the deployment step
  primary key (deployment_sequence_from), -- All transitions are 1:1
  constraint fk_from foreign key(deployment_sequence_from) references deployment_step(deployment_step_id),
  constraint fk_to foreign key(deployment_sequence_to) references deployment_step(deployment_step_id)
);

insert into deployment_sequence (deployment_sequence_from, deployment_sequence_to)
values
  ((select deployment_step_id from deployment_step where deployment_step_label = 'build'), (select deployment_step_id from deployment_step where deployment_step_label = 'beta')),
  ((select deployment_step_id from deployment_step where deployment_step_label = 'beta'), (select deployment_step_id from deployment_step where deployment_step_label = 'prod'))
;

drop type if exists deployment_status cascade;
create type deployment_status as enum ('SUCCESS', 'IN PROGRESS', 'FAILED');
drop table if exists deployment cascade;
create table deployment (
  deployment_id uuid unique primary key default gen_random_uuid(), -- The unique deployment ID
  deployment_project_id uuid not null, -- The project ID for the project getting updated
  deployment_step_id uuid not null, -- The unique step ID of this step that happened
  deployment_from_semver semver not null, -- The version that we are updating FROM
  deployment_to_semver semver not null, -- The version that we are updating TO
  deployment_start_time timestamp with time zone not null, -- The time of the beginning of the deployment step
  deployment_end_time timestamp with time zone not null, -- The time of the end of the deployment step
  deployment_status deployment_status not null, -- The deployment status ('SUCCESS', 'IN PROGRESS', 'FAILED')
  constraint fk_project foreign key (deployment_project_id) references project(project_id),
  constraint fk_step foreign key (deployment_step_id) references deployment_step(deployment_step_id)
);

drop table if exists registered_user cascade;
create table registered_user (
  user_id uuid unique primary key default gen_random_uuid(), -- The unique user ID
  user_name varchar(255) unique not null, -- The username for the user
  user_joined timestamp with time zone not null default now() -- The time that the user signed up
);

drop table if exists registered_user_email cascade;
create table registered_user_email (
  user_id uuid not null, -- The unique user ID
  user_email varchar(255) unique not null, -- The unique email for the user, all linked to the same account
  primary key (user_id, user_email),
  constraint fk_email foreign key (user_id) references registered_user(user_id)
);

insert into registered_user (user_name)
values
  ('kaspar'),
  ('nobody')
;

insert into registered_user_email (user_id, user_email)
values
  ((select user_id from registered_user where user_name = 'kaspar'), 'kaspar78@mouco.com'),
  ((select user_id from registered_user where user_name = 'kaspar'), 'polandkaspar78@gmail.com'),
  ((select user_id from registered_user where user_name = 'kaspar'), 'kaspar@typesofants.org'),
  ((select user_id from registered_user where user_name = 'nobody'), 'nobody@typesofants.org')
;

drop table if exists ant cascade;
create table ant (
  ant_id uuid unique primary key default gen_random_uuid(), -- Unique suggestion id
  ant_user_id uuid not null, -- The user that created the ant
  created_at timestamp with time zone not null default now(), -- when the ant was created
  suggested_content varchar(255) not null, -- The content that was originally suggested by the user.
  constraint fk_user foreign key (ant_user_id) references registered_user(user_id)
);

drop table if exists favorite cascade;
create table favorite (
  user_id uuid not null, -- The user who favorited the ant
  ant_id uuid not null, -- The ant that got favorited
  favorited_at timestamp with time zone not null default now(), -- When the favorite happened
  primary key (user_id, ant_id),
  constraint fk_user foreign key (user_id) references registered_user(user_id),
  constraint fk_ant foreign key (ant_id) references ant(ant_id)
);

drop table if exists ant_tweeted cascade;
create table ant_tweeted (
  ant_id uuid not null, -- The ant that got tweeted
  tweeted_at timestamp with time zone not null default now(), -- When the ant got tweeted
  primary key (ant_id, tweeted_at),
  constraint fk_ant foreign key (ant_id) references ant(ant_id)
);

drop table if exists ant_declined cascade;
create table ant_declined (
  ant_id uuid primary key not null, -- The ant that got declined
  ant_declined_user_id uuid not null, -- The person that declined the ant
  ant_declined_at timestamp with time zone not null default now(), -- When the ant was declined
  constraint fk_ant foreign key (ant_id) references ant(ant_id),
  constraint fk_user foreign key (ant_declined_user_id) references registered_user(user_id)
);

drop table if exists release cascade;
create table release (
  release_number serial primary key not null, -- The release ID
  release_label varchar(255) not null -- The name of the release, e.g. "1" or "initial release" or "birthday release"
);

drop table if exists ant_release cascade;
create table ant_release (
  release_number int not null, -- The current release version, always serial, e.g. 58
  ant_content_hash int not null, -- The hashed value of ant_content field. Used for ordering the ants. 32 bit integer.
  ant_content varchar(255) not null, -- The content of the suggestion, on release. This may differ from the original suggestion.
  ant_id uuid not null, -- The ant that was released in this version
  constraint fk_ant foreign key (ant_id) references ant(ant_id),
  constraint fk_release foreign key (release_number) references release(release_number),
  primary key (release_number, ant_id)
);

COMMIT;
