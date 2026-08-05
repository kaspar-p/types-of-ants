BEGIN;

-- Allow the codec to have any valid varchar(20) value
alter table archive_object
drop constraint archive_object_codec_check;

-- Since we have versions, it's (bucket, key, version)
alter table archive_object
drop constraint uq_archive_object_bucket_key;

drop table archive_object_placement;

-- Size is now based on the underlying chunks sized together.
alter table archive_object
alter column size_bytes drop not null;

-- Add a "key" table to handle the fact that a single user key might point to multiple versions,
-- at least 2 during an upload.
create table archive_key (
    key_id text primary key default ('key-' || random_string(24)),

    bucket_id text not null,
    key text not null,

    unique (bucket_id, key),

    current_object_id text,
    
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    deleted_at timestamp with time zone,

    foreign key (bucket_id) references archive_bucket(bucket_id),
    foreign key (current_object_id) references archive_object(object_id)
);

-- Instead of storing (object, key) right on the archive_object, point to the archive_key.
alter table archive_object
add column key_id text references archive_key(key_id);

alter table archive_object
alter column bucket_id drop not null;

alter table archive_object
alter column key drop not null;

-- Migrate all current objects to have archive_key entries as well
insert into archive_key
    (bucket_id, key, current_object_id)
select bucket_id, key, object_id
from archive_object
where deleted_at is null
on conflict
    (bucket_id, key)
do nothing;

-- Backfill their pointers to archive_key
update archive_object o
set key_id = k.key_id
from archive_key k
where
    k.bucket_id = o.bucket_id and
    k.key = o.key
;

-- Add chunk_strategy and redundancy_strategy to the object, chosen when it's PUT
alter table archive_object
add column chunk_strategy text not null default 'no_chunk';

alter table archive_object
add column redundancy_strategy text not null default 'replication';

-- Add nonce_prefix as bytes, which along with the (unencrypted) DEK will decrypt a chunk.
alter table archive_object
add column nonce_prefix bytea not null default '\x00'::bytea;

-- An object might be 10GB, and is split into many on-disk chunks for streaming and reading easier.
create table archive_chunk(
    chunk_id text primary key default ('ch-' || random_string(34)),
    
    object_id text not null, -- The object that this chunk was derived from.

    -- The ordering of the chunks. For simple encoding schemes (no ECC), the object is just the decrypted-then-concatenated chunks.
    chunk_index int not null,

    unique (object_id, chunk_index), -- The same object is not allowed to have multiple of e.g. "chunk 0".

    -- Whether the chunk has been completed (all shards replicated)
    is_complete bool not null default false,

    -- The size of the chunk, should be less than
    chunk_size_bytes int not null,

    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    deleted_at timestamp with time zone,

    foreign key (object_id) references archive_object(object_id)
);

-- Create a new table to sit between archive_object (a single key/value the customer sends) and archive_object_placement,
-- this represents a single SHARD of data being replicated to a single PLACE.
-- The shard is actually roughly chunk-sized, so points to a chunk instead of the object.
create table archive_shard(
    shard_id text primary key default ('shrd-' || random_string(36)),

    -- The chunk of data that we are sharding across failure domains.
    chunk_id text not null,

    -- The size of the shard of data. This matches most-closely with the size of data on-disk on the storage nodes.
    -- Might be exactly the size of the chunk (N-replication), or might be just the order of magnitude, since some
    -- replication schemes like Reed-Solomon ECC produce shards that are slightly larger than their data chunks.
    shard_size_bytes bigint not null,

    -- Redundancy scheme specific ordering. For example, replication is a simple counter and not valuable
    -- But for ECC(k, m), the shard_index goes from 0..(k+m-1), where the first 0..(k-1) indices are the data shards, and
    -- then the parity shards are the rest of the range.
    shard_index int not null,

    -- The checksum of the data in this shard. Since this is what goes to fallible storage nodes, for repair.
    shard_checksum bytea not null,

    unique (chunk_id, shard_index), -- The same chunk cannot have multiple e.g. "shard 0".

    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    deleted_at timestamp with time zone,

    foreign key (chunk_id) references archive_chunk(chunk_id)
);

-- Deprecates the archive_object_placement table, since OBJECTS no longer have a placement, only SHARDS do.
create table archive_placement(
    placement_id text primary key default('plc-' || random_string(16)),

    shard_id text not null, -- The shard that was placed on this node.
    storage_node_id text not null, -- The node that the shard was placed on.

    unique(shard_id, storage_node_id), -- The same shard cannot be on the same node more than once. 

    -- The actual key that was saved into the storage-node, for retrieving the object again.
    storage_key text not null,

    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now(),
    deleted_at timestamp with time zone,

    foreign key (shard_id) references archive_shard(shard_id),
    foreign key (storage_node_id) references archive_storage_node(storage_node_id)
);

insert into migration (migration_label) values ('support-object-sharding');

COMMIT;
