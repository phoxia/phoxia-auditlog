//! Minimal example: Axum app with audit logging.
//!
//! Run with:
//! ```bash
//! DATABASE_URL="postgres://postgres:postgres@localhost/idlefair" cargo run --example basic
//! ```

use axum::{routing::get, Router};
use phoxia_auditlog::{AuditConfig, AuditContext, AuditLayer};
use sqlx::PgPool;

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    audit: AuditContext,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost/idlefair".into()
    });

    let pool = PgPool::connect(&database_url).await?;

    // Create the audit table (in production, run via migrations)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id BIGSERIAL PRIMARY KEY,
            user_id UUID,
            action TEXT NOT NULL,
            ip TEXT,
            method TEXT,
            path TEXT,
            status SMALLINT,
            latency_ms INT,
            metadata JSONB,
            service_name TEXT NOT NULL DEFAULT 'example',
            ts TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(&pool)
    .await?;

    let config = AuditConfig::new(pool, "example-app");
    let (layer, ctx) = AuditLayer::new(config);
    let state = AppState { audit: ctx };

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .layer(layer)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> &'static str {
    "Hello, Phoxia!"
}

async fn health() -> &'static str {
    "OK"
}
