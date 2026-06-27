# phoxia-auditlog

Rust library published on crates.io. Phoxia projects import it to get automatic
audit trails via Tower middleware. Not a service — a library.

## Stack

| Layer | Choice |
|-------|--------|
| Language | Rust (pure library, no server) |
| Framework integration | Tower middleware (`tower-layer`) |
| Database writes | `sqlx` batch inserts directly to PostgreSQL |
| Serialization | `serde` + `serde_json` (metadata as JSONB) |

## Installation

```toml
[dependencies]
phoxia-auditlog = "0.1"
```

Or via `cargo add phoxia-auditlog`.

## API

```rust
use phoxia_auditlog::{AuditLayer, AuditConfig, audit};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;

    let config = AuditConfig::new(pool, "phoxia-id")
        .skip_path("/health")
        .skip_path("/metrics");

    let (layer, _ctx) = AuditLayer::new(config);

    let app = Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .layer(layer);

    // axum::serve(app).await
    Ok(())
}

// In handlers: explicit audit events
async fn update_user(
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserPayload>,
) -> impl IntoResponse {
    let user = state.db.update_user(&payload).await;

    audit!(state.audit, "user.updated", {
        "fields": ["email", "name"],
    });

    StatusCode::OK
}
```

## What is captured automatically (middleware)

| Field | Source |
|-------|--------|
| `user_id` | PhoxiaID JWT `sub` claim |
| `action` | `{method}:{path}` (e.g. `POST:/api/users`) |
| `ip` | `X-Forwarded-For` or socket addr |
| `method` | HTTP method |
| `path` | Request path |
| `status` | Response status code |
| `latency_ms` | Request duration |
| `ts` | Current timestamp |

## Trait `Auditable` (diff capture)

```rust
use phoxia_auditlog::Auditable;

#[derive(Debug, Serialize, Auditable)]
struct User {
    id: Uuid,
    email: String,
    role: String,
}

// After update:
// audit_diff!(state.audit, auth, "user.updated", &old_user, &new_user);
// → stores {"before": {...}, "after": {...}, "changed": ["email"]}
```

## Schema (created by the importing project)

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID,
    action TEXT NOT NULL,
    ip TEXT,
    method TEXT,
    path TEXT,
    status SMALLINT,
    latency_ms INT,
    metadata JSONB,
    service_name TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Note: no FK on user_id (deleted user doesn't erase history)
```

## Design decisions

- **Batch insert:** accumulates 100ms or 50 events, then flushes to PostgreSQL via sqlx. Never blocks the request.
- **Append-only schema:** no UPDATE/DELETE on audit_log table by design.
- **No FK on user_id:** user deletion doesn't cascade to audit history.
- **No index on ts:** append-only table, queries are rare and by user_id.
- **`skip_paths`:** health/metrics endpoints excluded to avoid noise.
- **Service name:** disambiguates audit entries when multiple services share a DB.

## Commands

```bash
cargo build                # build lib
cargo test                 # unit tests
cargo clippy               # lint
cargo doc --open           # documentation
cargo publish              # publish to crates.io
```

## Convention

- Docs in English
- Examples for every public API item
- Semver: breaking changes bump major
- Integration tests against a real Postgres (testcontainers or Docker)
