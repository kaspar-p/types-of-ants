begin;

insert into architecture
  (architecture_id, rust_target, docker_platform)
values
  ('raspbian', 'armv7-unknown-linux-gnueabihf', 'linux/arm64'),
  ('arm', 'aarch64-unknown-linux-gnu', 'linux/arm64'),
  ('x86', 'x86_64-unknown-linux-gnu', 'linux/amd64')
;

insert into host
  (host_id, architecture_id)
values
  ('antworker000.hosts.typesofants.org', 'raspbian'),
  ('antworker001.hosts.typesofants.org', 'arm'),
  ('antworker002.hosts.typesofants.org', 'arm'),
  ('antworker003.hosts.typesofants.org', 'arm'),
  ('antworker004.hosts.typesofants.org', 'arm'),
  ('ant.hisbaan.com', 'x86'),
  ('ant.flower.beer', 'x86')
;

insert into project
  (project_id, owned)
values
  ('ant-data-farm', true),
  ('ant-fs', true),
  ('ant-looking-pretty', true),
  ('ant-on-the-web', true),
  ('ant-backing-it-up', true),
  ('ant-backing-it-up-storage', true),
  ('ant-host-agent', true),
  ('ant-naming-domains', true),
  ('ant-who-tweets', true),
  ('ant-gateway', true),
  ('nextcloud-webdav', false)
;

insert into project_instance
  (project_id, host_id, environment, deployment_version)
values
  -- PROD 000
  ('ant-host-agent', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-data-farm', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-who-tweets', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-backing-it-up-storage', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  
  -- PROD 001
  ('ant-host-agent', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-backing-it-up', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-gateway', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-looking-pretty', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-on-the-web', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-naming-domains', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  
  -- PROD 003
  ('ant-host-agent', 'antworker003.hosts.typesofants.org', 'prod', 'live'),
  ('ant-backing-it-up-storage', 'antworker003.hosts.typesofants.org', 'prod', 'live'),
  
  -- PROD 004
  ('ant-host-agent', 'antworker004.hosts.typesofants.org', 'prod', 'live'),
  ('ant-fs', 'antworker004.hosts.typesofants.org', 'prod', 'live'),
  
  -- PROD other
  ('ant-fs', 'ant.hisbaan.com', 'prod', 'live'),
  ('nextcloud-webdav', 'ant.flower.beer', 'prod', 'live'),

  -- BETA
  ('ant-host-agent', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-backing-it-up-storage', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-backing-it-up', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-looking-pretty', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-on-the-web', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-data-farm', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-gateway', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-naming-domains', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-fs', 'antworker002.hosts.typesofants.org', 'beta', 'live')
;

commit;
