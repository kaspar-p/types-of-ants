BEGIN;

create user monitoring with login password 'to-be-replaced';

grant connect on database typesofants to monitoring;
grant usage on schema public to monitoring;
grant select on all tables in schema public to monitoring;
alter default privileges in schema public grant select on tables to monitoring;
grant pg_monitor to monitoring;

insert into migration (migration_label) values ('add-monitoring-user');

COMMIT;
