BEGIN;

alter table release
  add created_at timestamp with time zone, -- When the release was created.
  add updated_at timestamp with time zone -- When the release was last updated.
;

update release
set 
  created_at = '2022-06-05 00:00:00'::timestamp with time zone,
  updated_at = '2022-06-05 00:00:00'::timestamp with time zone
where release_number < 30
;

update release
set 
  created_at = '2023-08-08 00:00:00'::timestamp with time zone,
  updated_at = '2023-08-08 00:00:00'::timestamp with time zone
where release_number = 31 or release_number = 30
;

update release
set 
  created_at = '2024-02-03 00:00:00'::timestamp with time zone,
  updated_at = '2024-02-03 00:00:00'::timestamp with time zone
where release_number = 32
;

update release
set 
  created_at = '2024-04-14 00:00:00'::timestamp with time zone,
  updated_at = '2024-04-14 00:00:00'::timestamp with time zone
where release_number = 33
;

update release
set 
  created_at = '2024-09-08 00:00:00'::timestamp with time zone,
  updated_at = '2024-09-08 00:00:00'::timestamp with time zone
where release_number = 34
;

update release
set 
  created_at = '2025-02-17 00:00:00'::timestamp with time zone,
  updated_at = '2025-02-17 00:00:00'::timestamp with time zone
where release_number = 35
;

alter table release
alter column created_at set not null,
alter column created_at set default now(),
alter column updated_at set not null,
alter column updated_at set default now()
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-release-dates', now(), now())
;

COMMIT;
