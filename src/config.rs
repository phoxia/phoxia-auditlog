use sqlx::PgPool;
use std::collections::HashSet;

/// Configuration for the audit middleware.
///
/// # Example
///
/// ```rust,no_run
/// # use phoxia_auditlog::AuditConfig;
/// # use sqlx::PgPool;
/// # fn example() -> Result<(), sqlx::Error> {
/// let pool = PgPool::connect_lazy("postgres://localhost")?;
/// let config = AuditConfig::new(pool, "phoxia-id")
///     .skip_path("/internal/health")
///     .batch_size(100);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AuditConfig {
    /// sqlx connection pool. Required.
    pub pool: PgPool,

    /// Identifies this service in audit entries.
    /// e.g. `"phoxia-id"`, `"phoxia-watch"`, `"idle-fair"`.
    pub service_name: String,

    /// Name of the audit log table. Default: `"audit_log"`.
    pub table_name: String,

    /// Paths excluded from automatic audit. Case-sensitive exact match.
    /// Default: `{"/health", "/metrics"}`.
    pub skip_paths: HashSet<String>,

    /// Max events in buffer before forced flush. Default: `50`.
    pub batch_size: usize,

    /// Max milliseconds between flushes. Default: `100`.
    pub flush_interval_ms: u64,
}

impl AuditConfig {
    /// Create a config with sensible defaults.
    /// Only `pool` and `service_name` are required.
    pub fn new(pool: PgPool, service_name: impl Into<String>) -> Self {
        let mut skip_paths = HashSet::new();
        skip_paths.insert("/health".into());
        skip_paths.insert("/metrics".into());

        Self {
            pool,
            service_name: service_name.into(),
            table_name: "audit_log".into(),
            skip_paths,
            batch_size: 50,
            flush_interval_ms: 100,
        }
    }

    /// Override the table name.
    pub fn table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = name.into();
        self
    }

    /// Add a single path to the skip list.
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.skip_paths.insert(path.into());
        self
    }

    /// Override the full skip list.
    pub fn skip_paths(mut self, paths: HashSet<String>) -> Self {
        self.skip_paths = paths;
        self
    }

    /// Override batch size (max events before forced flush).
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Override flush interval in milliseconds.
    pub fn flush_interval_ms(mut self, ms: u64) -> Self {
        self.flush_interval_ms = ms;
        self
    }
}
