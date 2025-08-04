BEGIN;

alter table verification_attempt
alter column one_time_code type varchar(256)
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('increase-otp-length-for-hash', now(), now())
;

COMMIT;
