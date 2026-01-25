BEGIN;

create schema typesofants;
set search_path to typesofants;
alter database typesofants set search_path to typesofants;

create or replace function random_string(int) returns varchar(64) as $$
  select substr(md5(random()::varchar(64)), 0, $1+1);
$$ language sql;

create table migration (
  migration_id text primary key default ('m-' || random_string(8)), -- Unique migration ID
  migration_seq serial not null, -- The sequence number, in order.
  migration_label varchar(255) unique not null, -- Human readable label
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now()
);

create table project (
  project_id text primary key not null, -- Identifiable label, e.g. 'ant-data-farm'

  owned boolean not null, -- Whether we own this project. True for 'ant-data-farm', false for some WebDAV deployments owning backups

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table architecture (
  architecture_id text primary key,

  rust_target varchar(255), -- The target name for the Rust cross compiler to consume.
  docker_platform varchar(255), -- The docker platform for publishing images meant for this host.
  prometheus_os varchar(255), -- The OS component in Prometheus installs
  prometheus_arch varchar(255) -- The Arch component in Prometheus installs
);

create table host (
  host_id text primary key,
  
  architecture_id text not null, -- The architecture of the host

  constraint fk_architecture foreign key (architecture_id) references architecture(architecture_id)
);

-- A generic revision/version of a project. May have multiple artifacts,
-- one per architecture of the destination hosts.
create table revision (
  revision_id text primary key default ('rev-' || random_string(10)),

  project_id text not null, -- The project this is a version of.
  deployment_version text not null, -- The version.

  unique (project_id, deployment_version),

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (project_id) references project(project_id)
);

create table artifact (
  artifact_id text primary key default ('a-' || random_string(10)),

  revision_id text not null, -- The revision of this project.
  architecture_id text, -- The architecture this project was built for. If NULL, is platform-agnostic.

  unique (revision_id, architecture_id), -- Cannot have multiple of the same project and version

  local_path text not null, -- The local filesystem path to the artifact, where it was saved.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (revision_id) references revision(revision_id),
  foreign key (architecture_id) references architecture(architecture_id)
);

create table project_instance (
  project_instance_id text primary key default ('proj-i-' || random_string(10)),

  artifact_id text not null, -- The artifact that's deployed.
  host_id text not null, -- The host it's deployed onto.

  environment varchar(16), -- "prod", "beta", "dev". Null means the project is environment-agnostic.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (artifact_id) references artifact(artifact_id),
  foreign key (host_id) references host(host_id)
);

create table deployment_pipeline (
  deployment_pipeline_id text primary key default ('pipe-' || random_string(10)),

  project_id text not null, -- The project that this pipeline deploys.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (project_id) references project(project_id)
);

create table host_group (
  host_group_id text primary key default ('hostgroup-' || random_string(10)),

  host_group_name text unique not null, -- Human readable name.
  host_group_description text,

  -- The environment deployed onto these hosts. One of "dev", "beta", or "prod".
  -- This selects the environment variables this host is given.
  environment text not null, 

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table host_group_host (
  host_group_id text not null, -- The semantic name of the host group, e.g. "ant-on-the-web/beta"
  host_id text not null, -- The host in the group.

  primary key (host_group_id, host_id),

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (host_group_id) references host_group(host_group_id),
  foreign key (host_id) references host(host_id)
);

-- create table lock(
--   lock_id text primary key default ('lock-' || random_string(32)),

--   target_id text not null, -- The resource being locked.
--   authority_id text not null, -- The authority who locked this resource.

--   created_at timestamp with time zone not null default now(),
--   updated_at timestamp with time zone not null default now(),
--   deleted_at timestamp with time zone
-- );

create table deployment_pipeline_stage (
  deployment_pipeline_stage_id text primary key default ('stage-' || random_string(10)),

  deployment_pipeline_id text not null, -- The pipeline this stage is a part of.
  stage_name text not null, -- The name of the stage, e.g. "beta" or whatever.

  unique (deployment_pipeline_id, stage_name), -- Stages unique in a pipeline.

  stage_order int not null, -- The order of the stages, lower is earlier. 0 is the first stage.
  unique (deployment_pipeline_id, stage_order), -- Two stages cannot have the same order in a pipeline.

  stage_type text not null, -- Either 'build' or 'deploy', right now.

  stage_type_deploy_host_group_id text, -- The hosts to deploy to, in this stage. Only if stage_type is 'deploy'.

  unique (deployment_pipeline_id, stage_name, stage_order), -- No two stages can have the same order.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  foreign key (deployment_pipeline_id) references deployment_pipeline(deployment_pipeline_id),
  foreign key (stage_type_deploy_host_group_id) references host_group(host_group_id)
);

create table deployment (
  deployment_id text primary key default ('d-' || random_string(16)),

  revision_id text not null, -- The project revision that was deployed to that stage.
  target_type text not null, -- The type of the target (stage, pipeline) that this deployment happened to.
  target_id text not null, -- The unique ID of the target.
  event_name text not null, -- The name of the deployment event, e.g. "host-started"

  unique(revision_id, target_id, event_name),

  created_at timestamp with time zone not null default now(),

  foreign key (revision_id) references revision(revision_id)
);

create table deployment_job (
  deployment_job_id text primary key default ('djob-' || random_string(16)),

  revision_id text not null, -- The project revision that was deployed to that stage.
  target_type text not null, -- The type of target ('stage', 'pipeline') for this job.
  target_id text not null, -- The target (stage, pipeline, ...) that this deployment happened to.
  event_name text not null, -- The name of the deployment event, e.g. "host-started"

  -- unique(revision_id, target_type, target_id, event_name),

  deployment_pipeline_id text not null, -- The deployment project this job is scheduled within.
  project_id text not null, -- The project this job is scheduled for

  is_success boolean, -- Once the job was finished, was it successful?
  finished_at timestamp with time zone, -- Once the job is finished, the timestamp it finished at.
  is_retryable boolean, -- This will be true if the job is marked as retryable, and then future iterations will try again.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),

  foreign key (revision_id) references revision(revision_id),
  foreign key (deployment_pipeline_id) references deployment_pipeline(deployment_pipeline_id),
  foreign key (project_id) references project(project_id)
);

create table secret (
  secret_id text primary key default ('secret-' || random_string(32)),
  
  secret_name text not null, -- Human-knowable name of the secret, e.g. "tls_cert" or "jwt".
  secret_version int not null, -- A sequential version of the secret.
  secret_environment varchar(16), -- "prod", "beta", ...
   
  -- Example: ('jwt', 1, 'prod') is unique but different from ('jwt', 2, 'prod')
  -- and they are both different from any beta secret like ('jwt', 1, 'beta')
  unique (secret_name, secret_version, secret_environment),

  valid_for_seconds int not null, -- How long the secret is still valid, after its created_at date, in seconds
  secret_value bytea not null, -- The value of the secret, in bytes. May need to be stringified to be read.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table project_secret (
  project_instance_id text not null, -- The project instance using this secret.
  secret_id text not null, -- The exact secret being used.

  primary key (project_instance_id, secret_id),

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  constraint fk_project foreign key (project_instance_id) references project_instance(project_instance_id),
  constraint fk_secret foreign key (secret_id) references secret(secret_id)
);

insert into migration (migration_label) values
  ('bootstrap-schema')
;

COMMIT;
