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
  ('ant.hisbaan.com', 'x86'),
  ('ant.flower.beer', 'x86')
;

insert into project
  (project_id, owned)
values
  ('ant-data-farm', true),
  ('ant-fs', true),
  ('ant-on-the-web', true),
  ('ant-host-agent', true),
  ('ant-naming-domains', true),
  ('ant-who-tweets', true),
  ('ant-gateway', true),
  ('nextcloud-webdav', false)
;

insert into project_instance
  (project_id, host_id, environment, deployment)
values
  -- PROD
  ('ant-data-farm', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-backing-it-up-storage', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-backing-it-up', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-who-tweets', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-gateway', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-host-agent', 'antworker000.hosts.typesofants.org', 'prod', 'live'),
  ('ant-host-agent', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-fs', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-naming-domains', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-on-the-web', 'antworker001.hosts.typesofants.org', 'prod', 'live'),
  ('ant-fs', 'ant.hisbaan.com', 'prod', 'live'),
  ('nextcloud-webdav', 'ant.flower.beer', 'prod', 'live'),

  -- BETA
  ('ant-data-farm', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-gateway', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-host-agent', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-fs', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-naming-domains', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-on-the-web', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-backing-it-up-storage', 'antworker002.hosts.typesofants.org', 'beta', 'live'),
  ('ant-backing-it-up', 'antworker002.hosts.typesofants.org', 'beta', 'live')
;

commit;
