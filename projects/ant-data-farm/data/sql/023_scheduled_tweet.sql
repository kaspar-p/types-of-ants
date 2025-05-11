BEGIN;

create table scheduled_tweet (
  scheduled_tweet_id uuid primary key default gen_random_uuid(), -- The unique queue item.
  scheduled_at timestamp with time zone unique not null, -- When the tweet should be sent.
  scheduled_by uuid not null,
  tweet_prefix varchar(256), -- Optional, text to put before the ants tweeted.
  tweet_suffix varchar(256), -- Optional, text to put after the ants get tweeted.
  created_at timestamp with time zone not null default now(), -- When the queue suggestion was made
  updated_at timestamp with time zone not null default now(), -- When the queue suggestion was last updated.
  deleted_at timestamp with time zone, -- Optional, when the queue suggestion was deleted. Don't tweet in this case.

  constraint fk_user foreign key (scheduled_by) references registered_user(user_id)
);

create table scheduled_tweet_ant (
  scheduled_tweet_id uuid not null, -- A foreign key back to the parent scheduled_tweet item.
  ant_id uuid not null, -- The ant to tweet when it's time. Expects that this ant is released.
  
  primary key (scheduled_tweet_id, ant_id),
  constraint fk_scheduled_tweet foreign key (scheduled_tweet_id) references scheduled_tweet(scheduled_tweet_id),
  constraint fk_ant foreign key (ant_id) references ant(ant_id)
);

insert into scheduled_tweet
  (scheduled_at, scheduled_by)
values
  (to_timestamp('05/11/2025 12:00', 'MM/DD/YYYY HH24:MI') at time zone 'utc', (select user_id from registered_user where user_name = 'kaspar' limit 1))
;

insert into scheduled_tweet_ant
  (scheduled_tweet_id, ant_id)
values
  ((select scheduled_tweet_id from scheduled_tweet limit 1), (select ant_id from ant where suggested_content = 'ant on the phone with its mom (mother''s day)' limit 1))
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('new-scheduled-tweets-tables', now(), now())
;

COMMIT;
