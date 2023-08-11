FROM postgres

COPY data/sql/01_bootstrap_schema.sql /docker-entrypoint-initdb.d/01.sql
COPY data/sql/02_ant.sql /docker-entrypoint-initdb.d/02.sql
COPY data/sql/03_release.sql /docker-entrypoint-initdb.d/03.sql
COPY data/sql/04_ant_release.sql /docker-entrypoint-initdb.d/04.sql
COPY data/sql/05_ant_tweeted.sql /docker-entrypoint-initdb.d/05.sql
COPY data/sql/06_ant_declined.sql /docker-entrypoint-initdb.d/06.sql
