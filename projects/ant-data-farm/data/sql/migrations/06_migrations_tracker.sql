BEGIN;

drop table if exists migration;
create table migration (
  migration_id uuid unique primary key default gen_random_uuid(), -- The unique Migration ID
  migration_seq serial not null, -- The sequence number, the order in which migrations were applied.
  migration_label varchar(255) not null -- Human readable label
);

insert into migration (migration_label)
values
  ('Add phone number to the registered_user table'),
  ('New ant release, 08-08-2023'),
  ('New ant release, 02-03-2024'),
  ('New ant release, 04-14-2024'),
  ('Add the three hosts into host table, add more info into host table'),
  ('Add the migration table')
;

COMMIT;
