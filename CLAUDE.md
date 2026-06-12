# types-of-ants

## Style: always follow pre-existing patterns

Pre-existing style always takes precedent. Before adding new code, look at how
nearby code is structured and match it exactly.

## Test naming (integration tests)

Name integration tests as `{api}_returns_{status_code}_{case}`, e.g.:

- `stripe_webhook_returns_400_missing_signature`
- `stripe_webhook_returns_200_issuing_authorization_request`

See `ant-host-agent/tests/integration/service.rs` for reference.

## Lint suppression

Never use `#[allow(...)]`. Fix the underlying issue instead.
