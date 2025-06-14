BEGIN;

drop table if exists verification; -- Delete the old one, we don't need it, poorly designed

create table verification_attempt (
  verification_id uuid primary key default gen_random_uuid(), -- The unique attempt ID, for multiple attempts.

  user_id uuid not null, -- The user verifying
  unique_key varchar(256) not null, -- The phone number, email, or other verification method.
  verification_method varchar(256), -- One of 'phone', 'email'
  
  created_at timestamp with time zone not null default now(), -- When the verification request was created.
  expiration_seconds bigint not null, -- Verification requests should expire after they are sent.
  
  one_time_code varchar(64) not null, -- The short code the user can type in for two-factor authentication.
  send_id varchar(256), -- When verification request is sent, record a unique send ID for integration with sms/email providers.
  
  verification_attempts integer not null default 0, -- The number of times the user has tried to submit a verification.

  is_verified boolean not null default false, -- Set to false at the beginning, and true when the user's method is verified.
  verified_at timestamp with time zone, -- When the verification request was completed, null if not yet completed.

  is_cancelled boolean not null default false, -- Set to false at the beginning, true if the user asks to resend.
  cancelled_at timestamp with time zone,

  constraint fk_user foreign key (user_id) references registered_user(user_id)
);

insert into migration (migration_label, created_at, updated_at)
  values
    ('add-verification-table-2', now(), now())
;

COMMIT;
