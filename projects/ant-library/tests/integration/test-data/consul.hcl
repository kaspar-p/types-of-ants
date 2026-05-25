datacenter = "testing-typesofants-testing"
log_level  = "INFO"
bind_addr  = "{{ GetPrivateIP }}"

leave_on_terminate = true
rejoin_after_leave = true
encrypt_verify_incoming = false
encrypt_verify_outgoing = false

encrypt = "DH3SvNpT/0lleg3qsV9Zu5+HUdLRiJCIguC8WRtE5n4="

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
