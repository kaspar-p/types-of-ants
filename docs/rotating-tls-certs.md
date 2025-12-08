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
  -d '{"domains": ["beta.typesofants.org"]}'
  http://localhost:3235/certs/cert
```

Make sure not to spam this request, the TPS is _very_ low for this service. It
will create files in the local directory, likely
`projects/ant-zookeeper/dev-fs/...`. The one named `.crt` is the cert, copy that
to secrets:

```bash
cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_crt_beta_typesofants_org.crt \
  secrets/beta/tls_cert.secret

cp \
  projects/ant-zookeeper/dev-fs/dev-fs/certs-db/15354801451977472747_key_beta_typesofants_org.key \
  secrets/beta/tls_key.secret
```

Then, replicate the secrets to the relevant `ant-host-agents`.

```bash
./scripts/replicate-all-secrets.sh beta
```

and finally, bounce the `ant-gateway` service to reload its secrets.

```sh
./scripts/deploy.sh ant-gateway beta
```

That's it!
