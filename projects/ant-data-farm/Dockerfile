FROM postgres

COPY data/sql/01_bootstrap_schema.sql /docker-entrypoint-initdb.d/01.sql
COPY data/sql/02_ant.sql /docker-entrypoint-initdb.d/02.sql
COPY data/sql/03_release.sql /docker-entrypoint-initdb.d/03.sql
COPY data/sql/04_ant_release.sql /docker-entrypoint-initdb.d/04.sql
COPY data/sql/05_ant_tweeted.sql /docker-entrypoint-initdb.d/05.sql
COPY data/sql/06_ant_declined.sql /docker-entrypoint-initdb.d/06.sql

COPY data/sql/migrations/01_add_phone.sql /docker-entrypoint-initdb.d/07.sql
COPY data/sql/migrations/02_08-08-2023_ant_release.sql /docker-entrypoint-initdb.d/08.sql
COPY data/sql/migrations/03_02-03-2024_ant_release.sql /docker-entrypoint-initdb.d/09.sql
COPY data/sql/migrations/04_14-04-2024_ant_release.sql /docker-entrypoint-initdb.d/10.sql
COPY data/sql/migrations/05_add_hosts.sql /docker-entrypoint-initdb.d/11.sql
COPY data/sql/migrations/06_migrations_tracker.sql /docker-entrypoint-initdb.d/12.sql
