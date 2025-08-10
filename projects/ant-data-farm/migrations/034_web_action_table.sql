BEGIN;

create table web_action (
  web_action_id uuid primary key default gen_random_uuid(),

  actor_token uuid not null, -- The tracking cookie in the user's browser
  actor_user uuid not null, -- The user associated with this action. If anonymous, the "nobody" user.

  web_action varchar(256) not null, -- What they did: "click", "hover", "visit", ...
  web_target_type varchar(256) not null, -- The type of the web_target, e.g. "page", "button", "div", ...
  web_target varchar(256) not null, -- The target of the action, an ID of a button, URL of a page.

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  constraint fk_user foreign key (actor_user) references registered_user(user_id)
);

insert into migration (migration_label, created_at, updated_at)
values
  ('web-action-table', now(), now())
;

COMMIT;
