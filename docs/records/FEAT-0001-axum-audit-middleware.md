# FEAT-0001: Axum audit middleware

Status: implemented

The crate captures auditable HTTP activity through Tower middleware and batch-writes append-oriented records to PostgreSQL without an additional service. Applications remain responsible for authentication context, event classification, retention and authorization to read records.

Failures must be observable and must not silently claim an event was persisted. Consumers need an explicit policy for fail-open versus fail-closed behavior.
