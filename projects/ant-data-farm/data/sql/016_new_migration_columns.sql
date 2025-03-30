BEGIN;

alter table migration
  add created_at timestamp with time zone, -- When the migration was created.
  add updated_at timestamp with time zone -- When the migration was last updated.
;

update migration
set 
  created_at = '2023-06-20 00:00:00'::timestamp with time zone,
  updated_at = '2023-06-20 00:00:00'::timestamp with time zone
where migration_label = 'Add phone number to the registered_user table'
;

update migration
set
  created_at = '2023-08-08 00:00:00'::timestamp with time zone,
  updated_at = '2023-08-08 00:00:00'::timestamp with time zone
where migration_label = 'New ant release, 08-08-2023'
;

update migration
set
  created_at = '2024-02-03 00:00:00'::timestamp with time zone,
  updated_at = '2024-02-03 00:00:00'::timestamp with time zone
where migration_label = 'New ant release, 02-03-2024'
;

update migration
set
  created_at = '2024-04-14 00:00:00'::timestamp with time zone,
  updated_at = '2024-04-14 00:00:00'::timestamp with time zone
where migration_label = 'New ant release, 04-14-2024'
;

update migration
set
  created_at = '2024-05-05 00:00:00'::timestamp with time zone,
  updated_at = '2024-05-05 00:00:00'::timestamp with time zone
where migration_label = 'Add the three hosts into host table, add more info into host table'
;

update migration
set
  created_at = '2024-05-05 00:00:00'::timestamp with time zone,
  updated_at = '2024-05-05 00:00:00'::timestamp with time zone
where migration_label = 'Add the migration table'
;

update migration
set
  created_at = '2024-07-22 00:00:00'::timestamp with time zone,
  updated_at = '2024-07-22 00:00:00'::timestamp with time zone
where migration_label = 'make migration label unique'
;

update migration
set
  created_at = '2024-07-22 00:00:00'::timestamp with time zone,
  updated_at = '2024-07-22 00:00:00'::timestamp with time zone
where migration_label = 'make migration label unique'
;

update migration
set
  created_at = '2024-09-08 00:00:00'::timestamp with time zone,
  updated_at = '2024-09-08 00:00:00'::timestamp with time zone
where migration_label = 'ant-release:2024.9.8'
;

update migration
set
  created_at = '2025-02-17 00:00:00'::timestamp with time zone,
  updated_at = '2025-02-17 00:00:00'::timestamp with time zone
where migration_label = 'ant-release:2025.2.17'
;

alter table migration
alter column created_at set not null,
alter column updated_at set not null
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('new-migration-created-at-column', now(), now())
;

COMMIT;