BEGIN;

alter table registered_user
add column updated_at timestamp with time zone not null default now()
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-user-updated-at-column', now(), now())
;

COMMIT;
