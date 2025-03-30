BEGIN;

alter table migration add unique (migration_label);

insert into migration (migration_label)
  values
    ('make migration label unique')
;

COMMIT;
