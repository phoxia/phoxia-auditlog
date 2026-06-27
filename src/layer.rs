use crate::config::AuditConfig;
use crate::context::AuditContext;
use crate::service::AuditMiddleware;
use std::sync::Arc;
use tower_layer::Layer;

/// Tower Layer that wraps every request with audit logging.
///
/// Create via [`AuditLayer::new`], which returns both the layer
/// (for your Axum router) and an [`AuditContext`] (for your handlers).
#[derive(Clone)]
pub struct AuditLayer {
    pub(crate) config: Arc<AuditConfig>,
    pub(crate) ctx: AuditContext,
}

impl AuditLayer {
    /// Create a new layer and its associated context handle.
    ///
    /// Returns `(layer, context)`. Add `layer` to your Axum router via `.layer()`.
    /// Clone `context` into your app state — handlers use it with `audit!`.
    ///
    /// This also spawns the background batch writer task.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use phoxia_auditlog::{AuditLayer, AuditConfig};
    /// # use sqlx::PgPool;
    /// # fn example(pool: PgPool) {
    /// let config = AuditConfig::new(pool, "my-service");
    /// let (layer, ctx) = AuditLayer::new(config);
    /// // Add `layer` to your Axum router: `app.layer(layer)`
    /// # }
    /// ```
    pub fn new(config: AuditConfig) -> (Self, AuditContext) {
        let (ctx, rx) = AuditContext::channel();
        let config = Arc::new(config);

        // Spawn the batch writer as a background task
        crate::writer::BatchWriter::new(&config, rx).spawn();

        let layer = Self {
            config,
            ctx: ctx.clone(),
        };

        (layer, ctx)
    }
}

impl<S> Layer<S> for AuditLayer {
    type Service = AuditMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuditMiddleware {
            inner: service,
            config: Arc::clone(&self.config),
            ctx: self.ctx.clone(),
        }
    }
}
