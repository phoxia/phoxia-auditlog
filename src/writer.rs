use crate::config::AuditConfig;
use crate::event::AuditEvent;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Background task that accumulates audit events and flushes them
/// to PostgreSQL in batches inside a single transaction.
pub struct BatchWriter {
    pool: PgPool,
    table_name: String,
    batch_size: usize,
    flush_interval: Duration,
    rx: mpsc::UnboundedReceiver<AuditEvent>,
}

impl BatchWriter {
    /// Create a new batch writer. Takes ownership of the receiver.
    pub fn new(config: &AuditConfig, rx: mpsc::UnboundedReceiver<AuditEvent>) -> Self {
        Self {
            pool: config.pool.clone(),
            table_name: config.table_name.clone(),
            batch_size: config.batch_size,
            flush_interval: Duration::from_millis(config.flush_interval_ms),
            rx,
        }
    }

    /// Spawn the writer as a background Tokio task.
    ///
    /// Returns a [`tokio::task::JoinHandle`] that resolves when the
    /// channel is closed and the final flush completes.
    pub fn spawn(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut buffer: Vec<AuditEvent> = Vec::with_capacity(self.batch_size);
            let mut ticker = interval(self.flush_interval);

            loop {
                tokio::select! {
                    // New event from the channel
                    maybe_event = self.rx.recv() => {
                        match maybe_event {
                            Some(event) => {
                                buffer.push(event);
                                if buffer.len() >= self.batch_size {
                                    Self::flush_one(&self.pool, &self.table_name, &mut buffer).await;
                                }
                            }
                            // Channel closed — flush remaining and exit
                            None => {
                                Self::flush_one(&self.pool, &self.table_name, &mut buffer).await;
                                return;
                            }
                        }
                    }
                    // Timer tick — flush if we have buffered events
                    _ = ticker.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_one(&self.pool, &self.table_name, &mut buffer).await;
                        }
                    }
                }
            }
        })
    }

    /// Drain the buffer and insert all events in a single transaction.
    async fn flush_one(pool: &PgPool, table_name: &str, buffer: &mut Vec<AuditEvent>) {
        if buffer.is_empty() {
            return;
        }

        let events = std::mem::take(buffer);
        let count = events.len();
        let result = Self::insert_batch(pool, table_name, &events).await;

        match result {
            Ok(_) => {
                tracing::debug!(count, table_name, "Flushed audit events");
            }
            Err(e) => {
                tracing::error!(count, table_name, error = %e, "Failed to flush audit events");
            }
        }
    }

    /// Insert a batch of events in a single transaction.
    async fn insert_batch(
        pool: &PgPool,
        table_name: &str,
        events: &[AuditEvent],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        for event in events {
            let query = format!(
                "INSERT INTO {} (user_id, action, ip, method, path, status, latency_ms, metadata, service_name, ts) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                table_name
            );

            sqlx::query(&query)
                .bind(event.user_id)
                .bind(&event.action)
                .bind(event.ip.map(|ip| ip.to_string()))
                .bind(&event.method)
                .bind(&event.path)
                .bind(event.status)
                .bind(event.latency_ms)
                .bind(&event.metadata)
                .bind(&event.service_name)
                .bind(event.ts)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AuditContext;
    use crate::config::AuditConfig;
    use sqlx::PgPool;

    /// Test that the batch writer drains its buffer on channel close.
    /// Uses a lazy pool (no real Postgres) — flush will fail gracefully
    /// but the writer must exit without panicking.
    #[tokio::test]
    async fn writer_drains_buffer_on_channel_close() {
        let pool = PgPool::connect_lazy("postgres://localhost/nonexistent")
            .expect("lazy pool always succeeds");

        let mut skip = std::collections::HashSet::new();
        skip.insert("/health".into());
        let config = AuditConfig {
            pool: pool.clone(),
            service_name: "test".into(),
            table_name: "audit_log".into(),
            skip_paths: skip,
            batch_size: 3,
            flush_interval_ms: 500,
        };

        let (ctx, rx) = AuditContext::channel();
        let writer = BatchWriter::new(&config, rx);
        let handle = writer.spawn();

        // Send 2 events (below batch threshold — must flush on close)
        ctx.send(AuditEvent::new("test.1", "test"));
        ctx.send(AuditEvent::new("test.2", "test"));

        // Drop the context — closes the channel
        drop(ctx);

        // Writer should flush remaining 2 events on channel close and exit
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            handle,
        )
        .await;

        assert!(result.is_ok(), "writer should exit after channel closes without panicking");
    }
}
