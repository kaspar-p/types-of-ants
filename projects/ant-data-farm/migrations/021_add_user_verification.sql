BEGIN;

create table verification (
  user_id uuid not null, -- The user verifying
  unique_key varchar(256) not null, -- The phone number, email, or other verification method.
  verification_method varchar(256), -- One of 'phone', 'email'
  
  created_at timestamp with time zone not null default now(), -- When the verification request was created.
  expires_at timestamp with time zone not null, -- Verification requests should expire after they are sent.
  one_time_code varchar(64) not null, -- The short code the user can type in for two-factor authentication.
  
  is_verified boolean not null, -- Set to false at the beginning, and true when the user's method is verified.
  verified_at timestamp with time zone, -- When the verification request was completed, null if not yet completed.

  primary key (user_id, unique_key, verification_method), -- The user, the email (for example), and the 'email' method.
  constraint fk_user foreign key (user_id) references registered_user(user_id)
);

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-verification-table-for-two-factor', now(), now())
;

COMMIT;
