BEGIN;

create schema if not exists typesofants;
set SEARCH_PATH to typesofants;

drop domain if exists semver cascade;
create domain semver as varchar(50);

drop table if exists project cascade;
create table project (
  project_id serial unique primary key, -- The unique project ID
  project_label varchar(255) not null, -- The human-readable label for the project
  project_live_semver semver not null -- The semver 'X.Y.Z' version of the project, currently
);

insert into project (project_id, project_label, project_live_semver) 
values 
  (1, 'ant-on-the-web', '1.0.0'),
  (2, 'ant-just-checking-in', '1.0.0'),
  (3, 'ant-data-farm', '1.0.0')
;

drop table if exists test cascade;
create table test (
  test_id serial unique primary key, -- The unique test ID
  test_label varchar(255) not null, -- The human-readable label for the test
  test_project_id int not null, -- The unique project ID for which project this test is testing
  constraint fk_project foreign key (test_project_id) references project(project_id)
);

insert into test (test_id, test_label, test_project_id)
values
  (1, 'Test the web server for shallow healthy pings', 1)
;

drop table if exists test_instance cascade;
create table test_instance (
  test_instance_id serial unique primary key, -- Unique test id
  test_instance_test_id int not null, -- The type of test that is currently running. Corresponds to a key in the 'test_type' table.
  test_instance_start_time timestamp not null, -- A timestamp for the beginning of the test
  test_instance_end_time timestamp not null, -- A timestamp for the end of the test
  test_instance_status boolean not null,   -- The pass/fail status of the test
  constraint fk_test foreign key (test_instance_test_id) references test(test_id)
);

drop table if exists host cascade;
create table host (
  host_id serial unique primary key, -- The unique machine ID
  host_label varchar(255) not null -- A human-readable label for the machine, like 'Kaspar's Raspberry Pi'
);

insert into host (host_id, host_label)
values
  (1, 'Kaspar''s Raspberry Pi')
;

drop table if exists project_instance cascade;
create table project_instance (
  project_instance_id serial unique primary key, -- A unique ID for this project instance, on this machine
  project_instance_project_id int not null, -- The project that is currently running
  project_instance_machine_id int not null, -- The machine the project is currently running on
  project_instance_filepath varchar(255) not null, -- The path to the executable of that project, currently running
  project_instance_semver semver not null, -- The 'X.Y.Z' semantic version of the project currently running
  constraint fk_project foreign key(project_instance_id) references project(project_id),
  constraint fk_machine foreign key(project_instance_machine_id) references host(host_id)
);

drop table if exists deployment_step cascade;
create table deployment_step (
  deployment_step_id serial unique primary key, -- The unique step ID
  deployment_step_label varchar(255) not null -- A human-readable label for the step, like 'prod', 'beta', or 'build'
);

insert into deployment_step (deployment_step_id, deployment_step_label)
values
  (1, 'build'),
  (2, 'beta'),
  (3, 'prod')
;

drop table if exists deployment_sequence;
create table deployment_sequence (
  deployment_sequence_from int not null, -- The starting state of the deployment step[]
  deployment_sequence_to int not null, -- The ending state of the deployment step
  primary key (deployment_sequence_from), -- All transitions are 1:1
  constraint fk_from foreign key(deployment_sequence_from) references deployment_step(deployment_step_id),
  constraint fk_to foreign key(deployment_sequence_to) references deployment_step(deployment_step_id)
);

insert into deployment_sequence (deployment_sequence_from, deployment_sequence_to)
values
  (1, 2),
  (2, 3)
;

drop type if exists deployment_status cascade;
create type deployment_status as enum ('SUCCESS', 'IN PROGRESS', 'FAILED');
drop table if exists deployment cascade;
create table deployment (
  deployment_id serial unique primary key, -- The unique deployment ID
  deployment_project_id int not null, -- The project ID for the project getting updated
  deployment_step_id int not null, -- The unique step ID of this step that happened
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
  user_id serial unique primary key, -- The unique user ID
  user_name varchar(255) unique not null, -- The username for the user
  user_joined timestamp with time zone not null default now() -- The time that the user signed up
);

drop table if exists registered_user_email cascade;
create table registered_user_email (
  user_id int not null, -- The unique user ID
  user_email varchar(255) unique not null, -- The unique email for the user, all linked to the same account
  primary key (user_id, user_email),
  constraint fk_email foreign key (user_id) references registered_user(user_id)
);

insert into registered_user (user_id, user_name)
values
  (1, 'kaspar')
;

insert into registered_user_email (user_id, user_email)
values
  (1, 'kaspar78@mouco.com'),
  (1, 'polandkaspar78@gmail.com'),
  (1, 'kaspar@typesofants.org')
;

drop type if exists suggestion_status cascade;
create type suggestion_status as enum ('LIVE', 'DECLINED', 'DEPRECATED', 'UNSEEN');
drop table if exists suggestion cascade;
create table suggestion (
  suggestion_id serial unique primary key, -- Unique suggestion id
  suggestion_user_id int not null, -- The user ID of the user that submitted the suggestion
  suggestion_content varchar(255) not null, -- The content of the suggestion
  submitted timestamp with time zone not null default now(), -- When the suggestion was submitted
  suggestion_status suggestion_status not null, -- The status of this suggestion: 'LIVE', 'DECLINED', 'DEPRECATED', 'UNSEEN'
  constraint fk_user foreign key (suggestion_user_id) references registered_user(user_id)
);

COMMIT;