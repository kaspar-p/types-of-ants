# types-of-ants

## Style: always follow pre-existing patterns

Pre-existing style always takes precedent. Before adding new code, look at how
nearby code is structured and match it exactly.

## Test naming (integration tests)

Name integration tests as `{api}_returns_{status_code}_{case}`, e.g.:

- `stripe_webhook_returns_400_missing_signature`
- `stripe_webhook_returns_200_issuing_authorization_request`

See `ant-host-agent/tests/integration/service.rs` for reference.

## TDD

Always write a failing test before changing the implementation. Never write a
test asserting buggy behavior — only assert the correct end-state. If a correct
test can't be made to fail, fix the testing infrastructure first.

## Naming

Never suffix types with `Info`, `Data`, `Details`, or `Object`. If it's a Job,
call it `Job`. If it's a Pipeline, call it `Pipeline`. The type IS the thing.

## Comments

Never use `// --- Section Title ---` style dividers. If code needs grouping,
use modules or ordering. Comments should explain why, not act as headings.

Don't use ligatures in comments, keep it ASCII.

## Lint suppression

Never use `#[allow(...)]`. Fix the underlying issue instead.
