# types-of-ants

## Style: always follow pre-existing patterns

Pre-existing style always takes precedent. Before adding new code, look at how
nearby code is structured and match it exactly.

## Routes (ant-on-the-web)

Routes live in `src/routes/<module>.rs` and expose a `pub fn router() -> ApiRouter`
function. The router is then nested in `lib.rs` via `.nest("/path", module::router())`.
Never register routes directly in `lib.rs`.

Example: `webhooks::router()` is nested as `.nest("/webhooks", webhooks::router())`,
which makes the Stripe endpoint reachable at `/api/webhooks/stripe`.

## Test naming (integration tests)

Name integration tests as `{api}_returns_{status_code}_{case}`, e.g.:

- `stripe_webhook_returns_400_missing_signature`
- `stripe_webhook_returns_200_issuing_authorization_request`

See `ant-host-agent/tests/integration/service.rs` for reference.
