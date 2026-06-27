use crate::config::AuditConfig;
use crate::context::AuditContext;
use crate::event::AuditEvent;
use http::{HeaderMap, Request, Response};
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower_service::Service;

/// Tower Service that captures request metadata and sends audit events
/// to the background batch writer after the response is produced.
#[derive(Clone)]
pub struct AuditMiddleware<S> {
    pub(crate) inner: S,
    pub(crate) config: Arc<AuditConfig>,
    pub(crate) ctx: AuditContext,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AuditMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Capture request metadata before moving req into the inner service
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let headers = req.headers().clone();
        let ip = extract_ip(&headers);

        let skip = self.config.skip_paths.contains(&path);
        let service_name = self.config.service_name.clone();
        let ctx = self.ctx.clone();

        let start = std::time::Instant::now();

        // Tower services require clone for multiplexing
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            let response: Response<ResBody> = inner.call(req).await?;

            if !skip {
                let latency_ms = start.elapsed().as_millis() as i32;
                let status = response.status().as_u16() as i16;

                let event = AuditEvent {
                    user_id: None,
                    action: format!("{}:{}", method, path),
                    ip,
                    method: Some(method),
                    path: Some(path),
                    status: Some(status),
                    latency_ms: Some(latency_ms),
                    metadata: None,
                    service_name,
                    ts: chrono::Utc::now(),
                };

                ctx.send(event);
            }

            Ok(response)
        })
    }
}

/// Extract client IP: X-Forwarded-For first, then fall back to nothing.
/// (Socket address is not available at the Tower layer — it's in Axum's
/// `ConnectInfo` extension, which requires additional setup.)
fn extract_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
}
