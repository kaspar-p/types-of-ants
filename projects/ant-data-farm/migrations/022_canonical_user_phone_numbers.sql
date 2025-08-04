BEGIN;

update registered_user
set user_phone_number = '+19704812142'
where user_phone_number = '9704812142'
;

update registered_user
set user_phone_number = '+12223334444'
where user_phone_number = '0000000000'
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('canonical-user-phone-numbers', now(), now())
;

COMMIT;
