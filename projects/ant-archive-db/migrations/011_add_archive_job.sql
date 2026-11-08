BEGIN;

-- Background jobs that any ant-archive instance may run, but only one at a time. An instance
-- holds the lease while it runs the job, and the lease expires so a crashed instance doesn't
-- block the job forever.
create table archive_job (
    job_id text primary key,

    lease_holder text,
    lease_expires_at timestamp with time zone,

    -- What the last run saw, so the next run (on any instance) can skip when nothing changed.
    last_activity timestamp with time zone,
    last_run_clean bool not null default false,

    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

insert into archive_job (job_id) values ('vacuum');

insert into migration (migration_label) values ('add-archive-job');

COMMIT;
