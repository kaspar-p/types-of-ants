BEGIN;

-- Create monitoring user with login but no create/alter privileges
create user monitoring with login password 'to-be-replaced';

-- Grant connect to the database
grant connect on database typesofants to monitoring;

-- Grant usage on all schemas
grant usage on schema public to monitoring;

-- Grant select on all existing tables
grant select on all tables in schema public to monitoring;

-- Grant select on future tables automatically
alter default privileges in schema public grant select on tables to monitoring;

-- Grant access to pg_monitor role for postgres stats views
-- (pg_stat_activity, pg_stat_user_tables, pg_stat_bgwriter, etc.)
grant pg_monitor to monitoring;

insert into migration (migration_label) values ('add-monitoring-user');

COMMIT;
