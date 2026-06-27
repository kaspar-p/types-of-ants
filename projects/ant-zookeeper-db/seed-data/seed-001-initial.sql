begin;

insert into architecture
  (
    architecture_id,
    rust_target,
    docker_platform,
    prometheus_os,
    prometheus_arch
  )
values
  ('armv7', 'armv7-unknown-linux-gnueabihf', 'linux/arm64', 'linux', 'armv7'),
  ('aarch64', 'aarch64-unknown-linux-gnu', 'linux/arm64', 'linux', 'arm64'),
  ('x86_64', 'x86_64-unknown-linux-gnu', 'linux/amd64', 'linux', 'amd64')
;

insert into host
  (host_id, architecture_id)
values
  ('antworker000', 'armv7'),
  ('antworker001', 'aarch64'),
  ('antworker002', 'aarch64'),
  ('antworker003', 'aarch64'),
  ('antworker004', 'aarch64'),
  ('antworker005', 'aarch64'),
  ('antworker006', 'aarch64'),
  ('antworker007', 'aarch64'),
  ('hisbaan01', 'x86_64'),
  ('greg01', 'x86_64')
;

insert into project
  (project_id, owned)
values
  ('ant-data-farm', true),
  ('ant-fs', true),
  ('ant-looking-pretty', true),
  ('ant-on-the-web', true),
  ('ant-backing-it-up', true),
  ('ant-backing-it-up-db', true),
  ('ant-host-agent', true),
  ('ant-naming-domains', true),
  ('ant-who-tweets', true),
  ('ant-worker-node-metrics-exporter', true),
  ('ant-measuring-the-database', true),
  ('ant-gateway', true),
  ('ant-zookeeper-db', true),
  ('ant-zookeeper', true),
  ('ant-monitor', true),
  ('ant-monitor-fe', true),
  ('ant-matchmaker', true),
  ('ant-matchmaker-infra', true),
  ('ant-lumberjack', true),
  ('ant-sawmill', true),
  ('ant-just-checking-in', true),
  ('ant-printing-press', true),
  ('ant-archive', true),
  ('ant-archive-storage', true),
  ('ant-archive-db', true),
  ('nextcloud-webdav', false)
;

-- insert into project_instance
--   (project_id, deployment_version)
-- values
--   -- PROD 000
--   ('ant-host-agent', 'antworker000', 'prod', 'live'),
--   ('ant-data-farm', 'antworker000', 'prod', 'live'),
--   ('ant-who-tweets', 'antworker000', 'prod', 'live'),
--   ('ant-backing-it-up-db', 'antworker000', 'prod', 'live'),
  
--   -- PROD 001
--   ('ant-host-agent', 'antworker001', 'prod', 'live'),
--   ('ant-backing-it-up', 'antworker001', 'prod', 'live'),
--   ('ant-gateway', 'antworker001', 'prod', 'live'),
--   ('ant-looking-pretty', 'antworker001', 'prod', 'live'),
--   ('ant-on-the-web', 'antworker001', 'prod', 'live'),
--   ('ant-naming-domains', 'antworker001', 'prod', 'live'),
  
--   -- PROD 003
--   ('ant-host-agent', 'antworker003', 'prod', 'live'),
--   ('ant-backing-it-up-db', 'antworker003', 'prod', 'live'),
  
--   -- PROD 004
--   ('ant-host-agent', 'antworker004', 'prod', 'live'),
--   ('ant-fs', 'antworker004', 'prod', 'live'),
  
--   -- PROD other
--   ('ant-fs', 'ant.hisbaan.com', 'prod', 'live'),
--   ('nextcloud-webdav', 'ant.flower.beer', 'prod', 'live'),

--   -- BETA
--   ('ant-host-agent', 'antworker002', 'beta', 'live'),
--   ('ant-backing-it-up-db', 'antworker002', 'beta', 'live'),
--   ('ant-backing-it-up', 'antworker002', 'beta', 'live'),
--   ('ant-looking-pretty', 'antworker002', 'beta', 'live'),
--   ('ant-on-the-web', 'antworker002', 'beta', 'live'),
--   ('ant-data-farm', 'antworker002', 'beta', 'live'),
--   ('ant-gateway', 'antworker002', 'beta', 'live'),
--   ('ant-naming-domains', 'antworker002', 'beta', 'live'),
--   ('ant-fs', 'antworker002', 'beta', 'live')
-- ;

commit;
