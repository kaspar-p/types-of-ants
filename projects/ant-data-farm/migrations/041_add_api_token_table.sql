BEGIN;

create table api_token (
  api_token_id uuid primary key default gen_random_uuid(), -- The unique identifier of this token. Doesn't mean anything, does not grant permissions.

  user_id uuid not null, -- The user that this API token authenticates as. Full permissions!
  api_token_hash varchar(128) unique not null, -- The hashed value of the API token that the user has. Hashed because an API token
                            -- is credential material!

  created_at timestamp with time zone not null default now(), -- When the token was granted.
  updated_at timestamp with time zone not null default now(), -- When the token was updated/rotated.
  deleted_at timestamp with time zone, -- When the token was revoked, these are no longer valid!

  constraint fk_user foreign key (user_id) references registered_user(user_id)
);

insert into migration (migration_label)
values
  ('add-api-token-table')
;

COMMIT;
