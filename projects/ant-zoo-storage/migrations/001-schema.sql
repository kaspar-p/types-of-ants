BEGIN;

create schema typesofants;
set search_path to typesofants;
alter database typesofants set search_path to typesofants;

create domain identifier as varchar(64);

create or replace function random_string(int) returns text as $$
  select substr(md5(random()::text), 0, $1+1);
$$ language sql;

create table migration (
  migration_id identifier primary key default ('m-' || random_string(8)), -- Unique migration ID
  migration_seq serial not null, -- The sequence number, in order.
  migration_label varchar(255) unique not null, -- Human readable label
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now()
);

create table project (
  project_id identifier primary key not null, -- Identifiable label, e.g. 'ant-data-farm'

  owned boolean not null, -- Whether we own this project. True for 'ant-data-farm', false for some WebDAV deployments owning backups

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table architecture (
  architecture_id identifier primary key,

  rust_target varchar(255), -- The target name for the Rust cross compiler to consume.
  docker_platform varchar(255) -- The docker platform for publishing images meant for this host.
);

create table host (
  host_id identifier primary key,
  
  architecture_id identifier not null, -- The architecture of the host

  constraint fk_architecture foreign key (architecture_id) references architecture(architecture_id)
);

create table project_instance (
  project_instance_id identifier primary key default ('proj-' || random_string(10)),

  project_id identifier not null, -- The project that's deployed.
  host_id identifier not null, -- The host it's deployed onto.

  environment varchar(16), -- "prod", "beta", "dev". Null means the project is environment-agnostic.

  deployment_version identifier not null, -- The current version that this instance is running.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  unique (project_id, host_id), -- Cannot deploy the same project onto the same host more than once.

  constraint fk_project foreign key (project_id) references project(project_id),
  constraint fk_host foreign key (host_id) references host(host_id)
);

create table secret (
  secret_id identifier primary key default ('secret-' || random_string(32)),
  
  secret_environment varchar(16), -- "prod", "beta", ...
  secret_name identifier not null, -- Human-knowable name of the secret, e.g. "tls_cert" or "jwt".
  secret_version int not null, -- A sequential version of the secret.
   
  -- Example: ('jwt', 1, 'prod') is unique but different from ('jwt', 2, 'prod')
  -- and they are both different from any beta secret like ('jwt', 1, 'beta')
  unique (secret_name, secret_version, secret_environment),

  valid_for interval not null, -- How long the secret is still valid, after its created_at date.
  secret_value bytea not null, -- The value of the secret, in bytes. May need to be stringified to be read.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone
);

create table project_secret (
  project_instance_id identifier not null, -- The project instance using this secret.
  secret_id identifier not null, -- The exact secret being used.

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
