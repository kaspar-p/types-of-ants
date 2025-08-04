BEGIN;

alter table host
add host_hostname varchar(256) unique,
add host_type varchar(32),
add host_os varchar(32),
add host_user varchar(32);

update host
set 
  host_label = 'antworker000',
  host_location = 'Kaspar''s house, on the bottom',
  host_hostname = 'antworker000.hosts.typesofants.org',
  host_type = 'Raspberry Pi',
  host_os = 'Rasbian',
  host_user = 'ant'
where host_label = 'Kaspar''s Raspberry Pi';

insert into host (host_label, host_location, host_hostname, host_type, host_os, host_user)
  values
    ('antworker001', 'Kaspar''s house, in the middle', 'antworker001.hosts.typesofants.org', 'Libre', 'Ubuntu', 'ant'),
    ('antworker002', 'Kaspar''s house, on top', 'antworker002.hosts.typesofants.org', 'Libre', 'Ubuntu', 'ant')
;

alter table host
alter column host_hostname set not null,
alter column host_type set not null,
alter column host_os set not null,
alter column host_user set not null;

COMMIT;
