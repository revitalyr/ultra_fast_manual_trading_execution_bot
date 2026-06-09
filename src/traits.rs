use crate::execution::prepared_orders::PreparedOrder;
use crate::execution::prepared_orders::PreparedOrders;
use crate::match_engine::MatchConfig;
use anyhow::Result;
use bytes::Bytes;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Trait for trading client implementations
/// Allows swapping between different exchanges or mock implementations
#[async_trait::async_trait]
pub trait TradingClient: Send + Sync {
    async fn submit_order(&self, order: &PreparedOrder) -> Result<Value>;

    async fn submit_prepared_order(
        &self,
        payload: &Bytes,
        signature: Option<&Bytes>,
    ) -> Result<Value>;

    async fn get_markets(&self) -> Result<Vec<Value>>;

    async fn get_orderbook(&self, market_id: &str) -> Result<Value>;

    async fn get_balance(&self) -> Result<Value>;

    async fn cancel_order(&self, order_id: &str) -> Result<Value>;
}

/// Trait for execution engine implementations
/// Allows swapping between different execution strategies or mock implementations
#[async_trait::async_trait]
pub trait ExecutionEngine: Send + Sync {
    fn update_prepared_orders(&self, match_id: &str, orders: Arc<PreparedOrders>);

    async fn execute_match(&self, match_id: &str) -> Result<()>;

    fn get_execution_sender(
        &self,
    ) -> mpsc::Sender<crate::execution::prepared_orders::ExecutionRequest>;
}

/// Trait for the match manager interface consumed by the UI layer.
/// Allows mocking MatchManager in dashboard tests without spawning engines.
#[async_trait::async_trait]
pub trait MatchManagerHandle: Send + Sync {
    async fn execute_match(&self, match_id: &str) -> Result<()>;
    fn get_match_configs(&self) -> Vec<Arc<MatchConfig>>;
}
