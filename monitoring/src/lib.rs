use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tokio::sync::RwLock;

mod metrics;
pub use metrics::BlockchainMetrics;

/// Metrics server state
pub struct MetricsServer {
    metrics: Arc<RwLock<BlockchainMetrics>>,
}

impl MetricsServer {
    pub fn new(metrics: Arc<RwLock<BlockchainMetrics>>) -> Self {
        Self { metrics }
    }
    
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .with_state(Arc::new(self));
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Metrics server listening on {}", addr);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

async fn metrics_handler(
    State(server): State<Arc<MetricsServer>>,
) -> impl IntoResponse {
    let metrics = server.metrics.read().await;
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry().gather();
    
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        buffer,
    )
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = BlockchainMetrics::new().unwrap();
        assert!(metrics.registry().gather().len() > 0);
    }
}
