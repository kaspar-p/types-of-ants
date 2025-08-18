BEGIN;

create table ant_comment (
  comment_id uuid unique primary key default gen_random_uuid(), -- A unique ID for this comment.
  ant_id uuid not null, -- The ant that this comment is referring to.

  github_comment_id varchar(128), -- If this comment came from github, link it!
  parent_comment_id uuid, -- If a reply in thread, the parent comment.

  user_id uuid not null, -- The user that left the comment.

  body varchar(1024) not null, -- The content of the comment

  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  constraint fk_user_id foreign key (user_id) references registered_user(user_id),
  constraint fk_ant_id foreign key (ant_id) references ant(ant_id),
  constraint fk_parent_comment_id foreign key (parent_comment_id) references ant_comment(comment_id)
);

alter table ant
  add column github_issue_id varchar(128) -- If the ant was submitted via GitHub, the issue ID that links back.
;

create table registered_user_github (
  user_id uuid not null, -- The user connected to that GitHub
  github_user_id varchar(256) not null, -- The user ID on GitHub
  
  github_login varchar(128) not null, -- The login of the user on GitHub, might be different from their display name.
  github_name varchar(128) not null, -- The name of the user of GitHub, their display name.
  
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now(),
  deleted_at timestamp with time zone,

  primary key (user_id, github_user_id),
  constraint fk_user_id foreign key (user_id) references registered_user(user_id)
);

insert into registered_user_github
  (user_id, github_user_id, github_login, github_name)
values
  ((select user_id from registered_user where user_name = 'nobody'), 'MDQ6VXNlcjE0MTAxMjQw', 'kaspar-p', 'kaspar poland'),
  ((select user_id from registered_user where user_name = 'kaspar'), 'MDQ6VXNlcjE0MTAxMjQw', 'kaspar-p', 'kaspar poland')
;

insert into migration (migration_label, created_at, updated_at)
values
  ('add-ant-comments-and-github-id-links', now(), now())
;

COMMIT;
