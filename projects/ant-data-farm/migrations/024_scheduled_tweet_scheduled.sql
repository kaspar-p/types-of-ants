BEGIN;

alter table scheduled_tweet
 add is_tweeted boolean not null default false, -- If the tweet has already been sent.
 add tweeted_at timestamp with time zone -- When the tweet was sent. If null, has not been tweeted yet.
;

update scheduled_tweet
set is_tweeted = true, tweeted_at = now()
where scheduled_by = (select user_id from registered_user where user_name = 'kaspar' limit 1)
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('scheduled-tweets-scheduled-column', now(), now())
;

COMMIT;
