# ant-archive-storage

The storage node service for `ant-archive`, stores bytes received.

## Protocol

It is WebDAV compatible for all point operations (`GET`, `PUT`, `DELETE`), but
supports only a flat namespace, no subdirectories.

However, it requires bytes that are encrypted, that is, the payload received
should be:

```txt
payload = cipher(cipher(plaintext, DEK), TEK)
```

where the "inner" is encrypted with the DEK, which is never exposed to the
storage node. Then, that is encrypted with a Transport Encryption Key (TEK) that
is ephemeral but just to ensure that the router service is sending only
encrypted content. The `payload` is directly stored and the TEK is discarded
from memory.

The main listening port is set via the `PORT` environment variable, not
optional.

## Metrics

There is a secondary HTTP server that serves Prometheus metrics from the
`METRICS_PORT` environment variable, not optional. The metrics are then at path
`/metrics`.

## Storage format

Similar to Git, stores files as _hashes_ of the input key, so an input key
`path/to/file.txt` (which has SHA256 `b2e9331d95ea683...`), gets stored in a
directory:

```fs
b2/
  e9/
    b2e9331d95ea683...
```

where directories are nested by the first two characters to prevent filesystem
slowdowns for massive directories.

The storage directory should be persisted if in a container (Docker volume) or
on local files. The directory is set by the `PERSIST_DIR` environment variable,
and all files will be stored underneath.

## Authentication

All callers are `Bearer` authenticated with username/password. Valid tokens are
loaded from a file (local file, Docker secret file, etc.), at
`$TYPESOFANTS_SECRET_DIR/ant_archive_storage_auth.secret`, where
TYPESOFANTS_SECRET_DIR is an environment variable set to the directory of the
secret. For Docker deployments, `/run/secrets`.

The `ant_archive_storage_auth.secret` file must be formatted as a
newline-delimited file of `username:base64(sha256(password))` entries, to avoid
storing plaintext passwords at rest. Each new line represents a new
authentication.

For example, the header:

```txt
Authorization: Bearer user:test-password
```

would match the file:

```txt
user:xjiDP2m7+zwmevoKdENIEkNrjwioH9Jjxr5ocd5PEmU=
```
