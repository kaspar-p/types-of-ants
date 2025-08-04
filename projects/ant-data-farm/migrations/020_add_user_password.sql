BEGIN;

alter table registered_user
  add password_hash varchar(512) -- The hash of the password. Use algorithms like argon2 that have the salt built in.
;

update registered_user
set password_hash = 'dummy'
where password_hash is null
;

alter table registered_user
  alter column password_hash set not null
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-user-password-hash-and-salt', now(), now())
;

COMMIT;
