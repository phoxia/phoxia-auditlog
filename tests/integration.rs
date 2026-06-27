//! Integration test: full Axum app with audit middleware.
//! Requires a running Postgres. Run with:
//!
//! ```bash
//! DATABASE_URL="postgres://postgres:postgres@localhost/idlefair" \
//!   cargo test --features integration --test integration
//! ```

#[cfg(feature = "integration")]
mod pg_tests {
    use axum::{routing::get, Router};
    use phoxia_auditlog::{AuditConfig, AuditContext, AuditLayer};
    use sqlx::PgPool;
    use std::sync::Arc;

    #[derive(Clone)]
    struct AppState {
        audit: AuditContext,
    }

    async fn test_handler() -> &'static str {
        "ok"
    }

    async fn health_handler() -> &'static str {
        "healthy"
    }

    #[tokio::test]
    async fn full_flow_requests_logged_to_db() {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost/idlefair".into()
        });

        let pool = PgPool::connect(&database_url)
            .await
            .expect("connect to test db");

        // Create table
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
                service_name TEXT NOT NULL DEFAULT 'test',
                ts TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
        )
        .execute(&pool)
        .await
        .expect("create table");

        // Build config
        let config = AuditConfig::new(pool.clone(), "integration-test")
            .batch_size(2)
            .flush_interval_ms(50);

        let (layer, ctx) = AuditLayer::new(config);
        let state = AppState { audit: ctx };

        // Build Axum app
        let app = Router::new()
            .route("/api/test", get(test_handler))
            .route("/health", get(health_handler))
            .layer(layer)
            .with_state(state);

        // Start server on random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give the server a moment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = reqwest::Client::new();

        // Request 1: /api/test — should be audited
        let resp = client
            .get(format!("http://{}/api/test", addr))
            .header("x-forwarded-for", "10.0.0.1")
            .send()
            .await
            .expect("request");
        assert_eq!(resp.status(), 200);

        // Request 2: /health — should be SKIPPED
        let resp = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .expect("request");
        assert_eq!(resp.status(), 200);

        // Wait for batch flush
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Verify: only /api/test was logged
        let rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT action, method, ip FROM audit_log ORDER BY ts",
        )
        .fetch_all(&pool)
        .await
        .expect("query");

        assert_eq!(
            rows.len(),
            1,
            "only /api/test should be logged, /health skipped. Found {} rows",
            rows.len()
        );
        assert_eq!(rows[0].0, "GET:/api/test");
        assert_eq!(rows[0].1.as_deref(), Some("GET"));
        assert_eq!(rows[0].2.as_deref(), Some("10.0.0.1"));

        // Cleanup
        sqlx::query("DROP TABLE IF EXISTS audit_log")
            .execute(&pool)
            .await
            .expect("drop");
    }
}
