BEGIN;

alter table registered_user
alter column user_phone_number drop not null;

create table registered_user_phone_number (
  user_id uuid not null, -- The unique user ID
  phone_number varchar(32) unique not null, -- The phone number of the user, in +19994442222 format
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  primary key (user_id, phone_number),
  constraint fk_user foreign key (user_id) references registered_user(user_id)
);

insert into registered_user_phone_number
  (user_id, phone_number)
select user_id, user_phone_number
from registered_user
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-phone-numbers-table', now(), now())
;

COMMIT;
