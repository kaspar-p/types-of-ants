BEGIN;

alter table host add unique (host_label);

insert into migration (migration_label, created_at, updated_at)
  values
    ('make-hosts-label-unique', now(), now())
;

COMMIT;
