BEGIN;

create or replace function random_string(int) returns varchar(64) as $$
  select substr(md5(random()::varchar(64)), 0, $1+1);
$$ language sql;

create table migration (
  migration_id text primary key default ('m-' || random_string(8)),
  migration_seq serial not null,
  migration_label varchar(255) unique not null,
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now()
);

create table archive_client (
  client_id varchar(64) primary key default ('c-' || random_string(12)),
  client_name varchar(256) unique not null,
  token_hash varchar(256) not null,
  created_at timestamp with time zone not null default now()
);

create table archive_bucket (
  bucket_id varchar(64) primary key default ('b-' || random_string(5)),
  client_id varchar(64) not null,
  is_default boolean not null default false,
  read_policy varchar(64) not null default 'private'
    check (read_policy in ('public', 'internal', 'private')),
  
  created_at timestamp with time zone not null default now(),
  
  constraint fk_archive_bucket_client foreign key (client_id) references archive_client(client_id)
);

create table archive_kek_version (
  kek_id varchar(64) primary key default ('kek-' || random_string(5)),
  is_active boolean not null default true,

  created_at timestamp with time zone not null default now(),
  retired_at timestamp with time zone
);

create table archive_storage_node (
  storage_node_id varchar(64) primary key default ('sn-' || random_string(5)),
  host_id varchar(256) unique not null,
  is_active boolean not null default true,

  created_at timestamp with time zone not null default now(),
  deactivated_at timestamp with time zone
);

create table archive_object (
  object_id varchar(64) primary key default ('obj-' || random_string(32)),
  bucket_id varchar(64) not null,
  kek_id varchar(64) not null,
  
  key varchar(4096) not null,
  
  size_bytes bigint not null,
  codec varchar(20) not null default 'replication'
    check (codec in ('replication', 'erasure')),
  encrypted_dek bytea not null,
  dek_nonce bytea not null,
  
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,
  
  constraint fk_archive_object_bucket foreign key (bucket_id) references archive_bucket(bucket_id),
  constraint fk_archive_object_kek foreign key (kek_id) references archive_kek_version(kek_id),
  constraint uq_archive_object_bucket_key unique (bucket_id, key)
);

create table archive_object_placement (
  placement_id varchar(64) primary key default ('plcmt-' || random_string(16)),
  object_id varchar(64) not null,
  storage_node_id varchar(64) not null,
  idx integer not null,

  role varchar(20) not null default 'replica'
    check (role in ('replica', 'data', 'parity')),
  
  storage_key varchar(4096) not null,
  object_checksum varchar(64) not null,
  
  constraint fk_archive_placement_object foreign key (object_id) references archive_object(object_id),
  constraint fk_archive_placement_node foreign key (storage_node_id) references archive_storage_node(storage_node_id),
  constraint uq_archive_placement_object_idx unique (object_id, idx)
);

insert into migration (migration_label) values ('archive-schema');

COMMIT;
