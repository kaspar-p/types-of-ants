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
  client_id varchar(64) primary key,
  client_name varchar(256) unique not null,
  token_hash varchar(256) not null,
  created_at timestamp with time zone not null default now()
);

create table archive_bucket (
  bucket_id varchar(64) primary key,
  client_id varchar(64) not null,
  is_default boolean not null default false,
  read_policy varchar(20) not null default 'private'
    check (read_policy in ('public', 'internal', 'private')),
  created_at timestamp with time zone not null default now(),
  constraint fk_archive_bucket_client foreign key (client_id) references archive_client(client_id)
);

create unique index archive_bucket_one_default_per_client
  on archive_bucket (client_id)
  where is_default = true;

create table archive_kek_version (
  kek_id varchar(64) primary key,
  is_active boolean not null default true,
  created_at timestamp with time zone not null default now(),
  retired_at timestamp with time zone
);

create table archive_storage_node (
  storage_node_id varchar(64) primary key,
  host_id varchar(256) unique not null,
  is_active boolean not null default true,
  created_at timestamp with time zone not null default now()
);

create table archive_blob (
  blob_id varchar(64) primary key,
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
  constraint fk_archive_blob_bucket foreign key (bucket_id) references archive_bucket(bucket_id),
  constraint fk_archive_blob_kek foreign key (kek_id) references archive_kek_version(kek_id),
  constraint uq_archive_blob_bucket_key unique (bucket_id, key)
);

create index archive_blob_bucket_id_idx on archive_blob (bucket_id);
create index archive_blob_kek_id_idx on archive_blob (kek_id) where deleted_at is null;
create index archive_blob_codec_idx on archive_blob (codec) where deleted_at is null;

create table archive_blob_placement (
  placement_id varchar(64) primary key,
  blob_id varchar(64) not null,
  storage_node_id varchar(64) not null,
  idx integer not null,
  role varchar(20) not null default 'replica'
    check (role in ('replica', 'data', 'parity')),
  storage_key varchar(4096) not null,
  checksum varchar(64) not null,
  constraint fk_archive_placement_blob foreign key (blob_id) references archive_blob(blob_id),
  constraint fk_archive_placement_node foreign key (storage_node_id) references archive_storage_node(storage_node_id),
  constraint uq_archive_placement_blob_idx unique (blob_id, idx)
);

create index archive_blob_placement_blob_id_idx on archive_blob_placement (blob_id);

insert into migration (migration_label) values ('archive-schema');

COMMIT;
