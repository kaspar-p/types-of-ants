BEGIN;

create table user_role(
  role_id uuid primary key default gen_random_uuid(), -- A unique role ID
  role_name varchar(255) unique not null -- A unique name for the role, like 'admin'.
);

insert into user_role
  (role_name)
values
  ('admin'),
  ('user')
;

alter table registered_user
  add role_id uuid,
  add constraint fk_role foreign key (role_id) references user_role(role_id)
;

update registered_user
set role_id = (select role_id from user_role where role_name = 'admin' limit 1)
where user_name = 'kaspar'
;

update registered_user
set role_id = (select role_id from user_role where role_name = 'user' limit 1)
where role_id is null
;

alter table registered_user
  alter column role_id set not null
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('give-users-roles', now(), now())
;

COMMIT;
