BEGIN;

alter table registered_user
add user_phone_number varchar(20) unique;

update registered_user
set user_phone_number = '9704812142'
where user_name = 'kaspar';

update registered_user
set user_phone_number = '0000000000'
where user_name = 'nobody';

alter table registered_user
alter column user_phone_number set not null;

COMMIT;
