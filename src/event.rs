use chrono::{DateTime, Utc};
use serde::Serialize;
use std::net::IpAddr;
use uuid::Uuid;

/// A single audit log entry. Created by the middleware or by the `audit!` macro.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    /// User ID extracted from the JWT. None if unauthenticated or no extractor configured.
    pub user_id: Option<Uuid>,
    /// Action name. Middleware sets this to `"{method}:{path}"`. Macro events use explicit names.
    pub action: String,
    /// Client IP (from `X-Forwarded-For` or socket address).
    pub ip: Option<IpAddr>,
    /// HTTP method. None for macro events.
    pub method: Option<String>,
    /// Request path. None for macro events.
    pub path: Option<String>,
    /// HTTP response status. None for macro events.
    pub status: Option<i16>,
    /// Request duration in milliseconds. None for macro events.
    pub latency_ms: Option<i32>,
    /// Arbitrary JSON metadata from the `audit!` macro.
    pub metadata: Option<serde_json::Value>,
    /// Service name that generated this event.
    pub service_name: String,
    /// Timestamp of the event (set at capture time).
    pub ts: DateTime<Utc>,
}

impl AuditEvent {
    /// Create a new event with the current timestamp.
    pub fn new(action: impl Into<String>, service_name: impl Into<String>) -> Self {
        Self {
            user_id: None,
            action: action.into(),
            ip: None,
            method: None,
            path: None,
            status: None,
            latency_ms: None,
            metadata: None,
            service_name: service_name.into(),
            ts: Utc::now(),
        }
    }
}
