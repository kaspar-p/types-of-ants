# Rotating TLS Certs

Start the `ant-zookeeper` project if it isn't already:

```bash
./scripts/run-dev.sh ant-zookeeper
```

For the domains you want to request, make the CURL request:

```bash
curl \
  -X POST \
  -H 'Content-Type: application/json' \
  -d '{"domains": ["beta.typesofants.org"], "environment": "beta"}' \
  http://localhost:3235/cert/cert
```

Make sure not to spam this request, the TPS is _very_ low for this service. It
will create files in the local directory, likely
`projects/ant-zookeeper/dev-fs/...`. The one named `.crt` is the cert, copy that
to secrets:

```bash
cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_crt_beta_typesofants_org.crt \
  projects/ant-zookeeper/dev-fs/dev-fs/secrets-db/beta/tls_cert.secret

cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_crt_beta_typesofants_org.crt \
  secrets/beta/tls_cert.secret

cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_key_beta_typesofants_org.key \
  projects/ant-zookeeper/dev-fs/dev-fs/secrets-db/beta/tls_key.secret

cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_key_beta_typesofants_org.key \
  secrets/beta/tls_key.secret
```

and finally, bounce the `ant-gateway` service to reload its secrets:

```sh
./scripts/build.sh ant-gateway
```

That's it!
