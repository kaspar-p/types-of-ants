# ant-archive

A file storage service for typesofants usage. The `ant-archive` is the routing
and webservice layer, receiving and authenticating user requests and streaming
reads from storage services. Stores file metadata in `ant-archive-db`.

## Encryption

Plaintext is given to the ant-archive service, which encrypts with a unique
per-object Data Encryption Key (DEK) and unique nonce. The nonce is prefixed
onto the bytes of the ciphertext and sent to the storage node, so it doesn't
have to be stored.

The DEK is stored in the database but encrypted with a Key Encryption Key (KEK),
so that we can rotate the DEKs without needing to stream large objects and
re-encrypt them, we can just re-encrypt the DEK.

## Terminology

- **Object**: The logical blob of bytes that a user read/writes to the service
  via PUT/GET
- **Chunk**: A 4MB slice of the object, purely to ensure that all reads/writes
  can be done with constant memory size.
- **Shard**: A chunk, but potentially many copies of that chunk (many _shards_)
  are sent to distinct storage nodes. Purely to service the redundancy scheme.

## Storage nodes

The `ant-archive-storage` project is the storage node service.
