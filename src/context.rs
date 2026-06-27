use crate::event::AuditEvent;
use tokio::sync::mpsc;

/// A clone-able handle to the audit event channel.
///
/// Clone this into your app state so handlers can use the `audit!` macro.
/// The sender is lossy (unbounded) — if the receiver is gone, events are
/// silently dropped and logged as a warning via tracing.
#[derive(Clone)]
pub struct AuditContext {
    tx: mpsc::UnboundedSender<AuditEvent>,
}

impl AuditContext {
    /// Create a new context and the corresponding receiver.
    /// The receiver should be passed to [`BatchWriter::spawn`].
    pub(crate) fn channel() -> (Self, mpsc::UnboundedReceiver<AuditEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    /// Send an audit event. Non-blocking — if the channel is closed,
    /// the event is dropped and logged.
    pub fn send(&self, event: AuditEvent) {
        if let Err(e) = self.tx.send(event) {
            tracing::warn!(
                action = %e.0.action,
                "Audit event dropped: batch writer channel closed"
            );
        }
    }
}
