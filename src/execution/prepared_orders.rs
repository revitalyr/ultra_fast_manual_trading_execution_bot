use anyhow::{anyhow, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PreparedOrder {
    pub id: Uuid,
    pub market_id: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub size: f64,
    pub price: Option<f64>,
    pub signature: Option<Bytes>,
    pub created_at: u64,
}

impl fmt::Debug for PreparedOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedOrder")
            .field("id", &self.id)
            .field("market_id", &self.market_id)
            .field("order_type", &self.order_type)
            .field("side", &self.side)
            .field("size", &self.size)
            .field("price", &self.price)
            .field("signature", &self.signature.as_ref().map(|_| "[REDACTED]"))
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl PreparedOrder {
    /// Creates a new PreparedOrder with order parameters. Payload is generated at execution time.
    pub fn with_params(
        market_id: String,
        order_type: OrderType,
        side: OrderSide,
        size: f64,
        price: Option<f64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            market_id,
            order_type,
            side,
            size,
            price,
            signature: None,
            created_at: crate::util::now_millis(),
        }
    }

    /// Creates a placeholder order for initialization. Not a valid order for execution.
    pub fn placeholder() -> Self {
        Self {
            id: Uuid::nil(),
            market_id: String::new(),
            order_type: OrderType::Market,
            side: OrderSide::Buy,
            size: 0.0,
            price: None,
            signature: None,
            created_at: 0,
        }
    }

    /// Generate execution payload with current timestamp.
    /// Validates all numeric fields are finite before serialization.
    /// Called at execution time to ensure fresh timestamp.
    pub fn build_payload(&self) -> Result<Bytes> {
        if !self.size.is_finite() || self.size <= 0.0 {
            return Err(anyhow!("Invalid order size: {}", self.size));
        }
        if let Some(price) = self.price {
            if !price.is_finite() || price <= 0.0 {
                return Err(anyhow!("Invalid order price: {}", price));
            }
        }
        if self.market_id.is_empty() {
            return Err(anyhow!("Empty market_id"));
        }

        let payload = json!({
            "marketId": self.market_id,
            "type": match self.order_type {
                OrderType::Market => "market",
                OrderType::Limit { price: _ } => "limit",
            },
            "side": match self.side {
                OrderSide::Buy => "buy",
                OrderSide::Sell => "sell",
            },
            "size": self.size,
            "price": self.price,
            "timestamp": crate::util::now_millis()
        });
        Ok(Bytes::from(serde_json::to_vec(&payload)?))
    }
}

#[derive(Debug, Clone)]
pub struct PreparedOrders {
    #[allow(dead_code)]
    pub match_id: String,
    pub goal_market_order: PreparedOrder,
    pub match_result_order: PreparedOrder,
    #[allow(dead_code)]
    // This field is useful for debugging/future features
    pub updated_at: u64,
}

impl PreparedOrders {
    pub fn new(
        match_id: String,
        goal_market_order: PreparedOrder,
        match_result_order: PreparedOrder,
    ) -> Self {
        Self {
            match_id,
            goal_market_order,
            match_result_order,
            updated_at: crate::util::now_millis(),
        }
    }
}

use crate::market_data::{OrderSide, OrderType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub order_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub match_id: String,
    pub orders: Arc<PreparedOrders>,
}

impl ExecutionRequest {
    pub fn new(match_id: String, orders: Arc<PreparedOrders>) -> Self {
        Self { match_id, orders }
    }
}
