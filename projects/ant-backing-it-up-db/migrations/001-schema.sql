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

create table backup (
  backup_id identifier primary key default ('b-' || random_string(16)), -- Unique ID for the backup.

  project varchar(255) not null, -- The project that was backed up, e.g. 'ant-data-farm'.
  database_host varchar(255) not null, -- The database host name, e.g. 'antworker001.hosts.typesofants.org'
  database_port int not null, -- The database host port, e.g. '7000'

  encryption_nonce bytea, -- The random "salt" stored for each file.
  
  destination_host varchar(255) not null, -- The destination ant-fs host name, e.g. 'antworker001.hosts.typesofants.org'
  destination_port int not null, -- The destination ant-fs host port, e.g. '7000'
  destination_filepath varchar(255) not null, -- The path on the ant-fs instance.

  created_at timestamp with time zone not null default now(), -- When the backup was started.
  updated_at timestamp with time zone not null default now(), -- When the backup was updated.
  deleted_at timestamp with time zone -- Null unless the backup was deleted.
);

insert into migration (migration_label) values
  ('bootstrap-schema')
;

COMMIT;
