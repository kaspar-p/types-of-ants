datacenter = "typesofants"
log_level  = "INFO"
bind_addr  = "{{ GetPrivateIP }}"

leave_on_terminate = true
rejoin_after_leave = true

ports {
  dns      = -1
  serf_wan = -1
  grpc     = -1
  grpc_tls = -1
}

telemetry {
  prometheus_retention_time = "60s"
}

performance {
  raft_multiplier = 1
}
