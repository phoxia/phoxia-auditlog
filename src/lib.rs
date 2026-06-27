//! phoxia-auditlog — Automatic audit logging for Axum apps.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use phoxia_auditlog::{AuditLayer, AuditConfig};
//! use sqlx::PgPool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PgPool::connect("postgres://...").await?;
//! let config = AuditConfig::new(pool, "my-service");
//! let (layer, _ctx) = AuditLayer::new(config);
//! // Add `layer` to your Axum router with `.layer(layer)`
//! # Ok(())
//! # }
//! ```

pub mod auditable;
pub mod config;
pub mod context;
pub mod event;
pub mod layer;
pub mod service;
pub mod writer;

pub use auditable::Auditable;
pub use config::AuditConfig;
pub use context::AuditContext;
pub use event::AuditEvent;
pub use layer::AuditLayer;

// Re-exported for the audit! and audit_diff! macros
pub use chrono;
pub use serde_json;

/// Log an explicit audit event from a handler.
///
/// # Usage
///
/// ```rust,no_run
/// # use phoxia_auditlog::audit;
/// # use phoxia_auditlog::AuditContext;
/// # // In real code, get the context from AuditLayer::new()
/// # // This is just a demonstration of the macro syntax
/// # fn example(ctx: AuditContext) {
/// audit!(ctx, "user.login", {
///     "method": "passkey",
///     "success": true,
/// });
/// # }
/// ```
///
/// The first argument is an [`AuditContext`]. The second is the action name.
/// The third is a JSON object literal using `serde_json::json!` syntax.
#[macro_export]
macro_rules! audit {
    // Form: audit!(ctx, "action.name", { key: value, ... })
    ($ctx:expr, $action:expr, { $($key:tt : $value:expr),* $(,)? }) => {
        {
            let event = $crate::AuditEvent {
                user_id: None,
                action: $action.into(),
                ip: None,
                method: None,
                path: None,
                status: None,
                latency_ms: None,
                metadata: Some($crate::serde_json::json!({ $($key: $value),* })),
                service_name: String::new(),
                ts: $crate::chrono::Utc::now(),
            };
            $crate::AuditContext::send(&$ctx, event);
        }
    };
    // Form: audit!(ctx, "action.name") — no metadata
    ($ctx:expr, $action:expr) => {
        {
            let event = $crate::AuditEvent {
                user_id: None,
                action: $action.into(),
                ip: None,
                method: None,
                path: None,
                status: None,
                latency_ms: None,
                metadata: None,
                service_name: String::new(),
                ts: $crate::chrono::Utc::now(),
            };
            $crate::AuditContext::send(&$ctx, event);
        }
    };
}

/// Log an audit event with before/after diff of a mutated value.
///
/// # Usage
///
/// ```rust,ignore
/// let old_user = state.db.get_user(id).await?;
/// let new_user = state.db.update_user(id, &payload).await?;
///
/// audit_diff!(state.audit, "user.updated", &old_user, &new_user);
/// ```
///
/// The metadata will contain `before`, `after`, and `changed` (list of field names).
/// (Extra metadata keys supported in a future version.)
#[macro_export]
macro_rules! audit_diff {
    ($ctx:expr, $action:expr, $old:expr, $new:expr) => {
        {
            let changed: std::collections::HashSet<String> =
                $crate::Auditable::changed_fields($old, $new);

            let metadata = $crate::serde_json::json!({
                "before": $crate::Auditable::to_audit_json($old),
                "after": $crate::Auditable::to_audit_json($new),
                "changed": changed.iter().collect::<Vec<_>>(),
            });

            let event = $crate::AuditEvent {
                user_id: None,
                action: $action.into(),
                ip: None,
                method: None,
                path: None,
                status: None,
                latency_ms: None,
                metadata: Some(metadata),
                service_name: String::new(),
                ts: $crate::chrono::Utc::now(),
            };
            $crate::AuditContext::send(&$ctx, event);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_macro_constructs_event_with_metadata() {
        let (ctx, mut rx) = AuditContext::channel();

        audit!(ctx, "user.login", {
            "method": "passkey",
            "success": true,
        });

        let event = rx.try_recv().expect("event should be sent");
        assert_eq!(event.action, "user.login");
        assert_eq!(
            event.metadata,
            Some(serde_json::json!({"method": "passkey", "success": true}))
        );
    }

    #[test]
    fn audit_macro_no_metadata() {
        let (ctx, mut rx) = AuditContext::channel();

        audit!(ctx, "health.check");

        let event = rx.try_recv().expect("event should be sent");
        assert_eq!(event.action, "health.check");
        assert!(event.metadata.is_none());
    }

    #[test]
    fn audit_diff_macro_includes_before_after_changed() {
        use crate::auditable::Auditable;
        use serde_json::Value;

        #[derive(Clone)]
        struct User {
            name: String,
        }
        impl Auditable for User {
            fn to_audit_json(&self) -> Value {
                serde_json::json!({"name": self.name})
            }
        }

        let (ctx, mut rx) = AuditContext::channel();
        let old = User { name: "Alice".into() };
        let new = User { name: "Bob".into() };

        audit_diff!(ctx, "user.renamed", &old, &new);

        let event = rx.try_recv().expect("event sent");
        assert_eq!(event.action, "user.renamed");

        let meta = event.metadata.expect("metadata present");
        assert_eq!(meta["before"]["name"], "Alice");
        assert_eq!(meta["after"]["name"], "Bob");
        assert!(meta["changed"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("name".into())));
    }
}
