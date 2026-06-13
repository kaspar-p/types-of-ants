BEGIN;

-- Seed the well-known "typesofants" client and its default buckets.
-- The token_hash must be replaced before production use (sha256 of the actual token, base64-encoded).
insert into archive_client (client_id, client_name, token_hash) values
  ('client-typesofants', 'typesofants', 'REPLACE_WITH_SHA256_BASE64_OF_TOKEN');

insert into archive_bucket (bucket_id, client_id, is_default, read_policy) values
  ('b-typesofants', 'client-typesofants', true, 'internal'),
  ('b-public',      'client-typesofants', false, 'public');

COMMIT;
