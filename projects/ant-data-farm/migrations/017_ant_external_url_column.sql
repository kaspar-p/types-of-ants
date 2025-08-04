BEGIN;

update ant_release
set ant_content = '[mouco ant](https://mouco.com)'
where ant_content = 'mouco ant'
;

update ant_release
set ant_content = '(ant that would love to work at amazon but now they actually work amazon :))[https://www.linkedin.com/in/kaspar-p/]'
where ant_content = 'ant that would love to work at amazon but now they actually work amazon :)'
;

update ant_release
set ant_content = '[[6krill] ant](https://6krill.com)'
where ant_content = '[6krill] ant'
;

update ant_release
set ant_content = 'ant on twitter! [@typesofants](https://twitter.com/typesofants)'
where ant_content = 'ant on twitter! @typesofants'
;

insert into migration (migration_label, created_at, updated_at)
  values
    ('ant-release-markdown-links', now(), now())
;

COMMIT;
