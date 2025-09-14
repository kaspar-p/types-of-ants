BEGIN;

alter table release
  add column creator_user_id uuid, -- The user that created the release.
  add constraint fk_user foreign key (creator_user_id) references registered_user(user_id)
;

update release
  set creator_user_id = (select user_id from registered_user where user_name = 'nobody')
;

alter table release
alter column creator_user_id set not null;

alter sequence release_release_number_seq restart with 37;

insert into migration (migration_label)
values
  ('release-has-creator-and-fix-sequence')
;

COMMIT;
