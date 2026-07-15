# Phoxia audit

Date: 2026-07-14. Confidence: high for source and tests; no deployed consumer was inspected.

## Evidence

- Rust Tower/Axum middleware writes structured audit events to PostgreSQL.
- Integration tests cover the middleware path.
- The crate is backend-only, so Lux and UI accessibility are not applicable.

## Result

- The public API is documented in the README, but operations, retention, privacy classification and compatibility were not explicit.
- `FEAT-0001` records the implemented middleware boundary.
- Event schema or semantics shared across Phoxia products require RFC analysis; local implementation choices use ADRs.

## Remaining evidence

- Define approved retention, deletion exceptions and access roles before production use with personal data.
- Prove database failure behavior and replay/flush behavior under shutdown.
