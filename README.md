# phoxia-auditlog

[![Crates.io](https://img.shields.io/crates/v/phoxia-auditlog)](https://crates.io/crates/phoxia-auditlog)
[![Docs](https://docs.rs/phoxia-auditlog/badge.svg)](https://docs.rs/phoxia-auditlog)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-purple.svg)](https://www.gnu.org/licenses/agpl-3.0)

Automatic audit logging for Axum apps. Tower middleware that captures every
HTTP request and writes it to PostgreSQL — zero external services.

## Features

- **Zero-config middleware** — wrap your Axum router, every request is logged
- **Batch writes** — accumulates events, flushes every 100ms or 50 events
- **Non-blocking** — background Tokio task, never slows down a request
- **`audit!` macro** — log explicit events from handlers with JSON metadata
- **`audit_diff!` macro** — capture before/after snapshots when data changes
- **`Auditable` trait** — implement on your types for automatic diff detection
- **Skip paths** — exclude `/health`, `/metrics`, or any path you choose
- **Service name** — disambiguate entries when multiple services share a DB

## Installation

```toml
[dependencies]
phoxia-auditlog = "0.1"
```

Requires PostgreSQL with the table below.

## Quick start

### 1. Create the audit table

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
```

### 2. Add the middleware

```rust
use phoxia_auditlog::{AuditConfig, AuditLayer};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPool::connect("postgres://...").await?;
    let config = AuditConfig::new(pool, "my-service");
    let (layer, _ctx) = AuditLayer::new(config);

    // Add to your Axum router
    // let app = Router::new().layer(layer);
    Ok(())
}
```

### 3. Log explicit events

```rust
use phoxia_auditlog::audit;

async fn delete_user(State(state): State<AppState>) -> impl IntoResponse {
    audit!(state.audit, "user.deleted", {
        "user_id": "abc-123",
    });
    StatusCode::NO_CONTENT
}
```

### 4. Log data changes with diff

```rust
use phoxia_auditlog::{audit_diff, Auditable};
use serde_json::Value;

#[derive(Clone)]
struct User { name: String, email: String }

impl Auditable for User {
    fn to_audit_json(&self) -> Value {
        serde_json::json!({ "name": self.name, "email": self.email })
    }
}

async fn update_user(State(state): State<AppState>) -> impl IntoResponse {
    let old = state.db.get_user(id).await?;
    let new = state.db.update_user(id, &payload).await?;

    audit_diff!(state.audit, "user.updated", &old, &new);
    StatusCode::OK
}
```

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `pool` | (required) | sqlx `PgPool` |
| `service_name` | (required) | Identifies this service in audit entries |
| `table_name` | `"audit_log"` | Target table name |
| `skip_paths` | `{"/health", "/metrics"}` | Paths excluded from automatic audit |
| `batch_size` | `50` | Max events before forced flush |
| `flush_interval_ms` | `100` | Max ms between flushes |

## Design

- **Append-only:** no UPDATE/DELETE on the audit table, ever
- **No FK on user_id:** deleted users don't erase audit history
- **Background writer:** flushes batches in a Tokio task — requests never block
- **AGPLv3:** open source, copyleft

## License

AGPLv3. See [LICENSE](LICENSE).
