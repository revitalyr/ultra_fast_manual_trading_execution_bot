use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedOrder {
    pub id: Uuid,
    pub market_id: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub size: f64,
    pub price: Option<f64>,
    pub payload: Bytes,
    pub signature: Option<Bytes>,
    pub created_at: u64,
}

impl PreparedOrder {
    pub fn new(
        market_id: String,
        order_type: OrderType,
        side: OrderSide,
        size: f64,
        price: Option<f64>,
        payload: Bytes,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            market_id,
            order_type,
            side,
            size,
            price,
            payload,
            signature: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_signature(mut self, signature: Bytes) -> Self {
        self.signature = Some(signature);
        self
    }
}

impl Default for PreparedOrder {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            market_id: String::new(),
            order_type: OrderType::Market,
            side: OrderSide::Buy,
            size: 0.0,
            price: None,
            payload: Bytes::new(),
            signature: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedOrders {
    pub match_id: String,
    pub goal_market_order: PreparedOrder,
    pub match_result_order: PreparedOrder,
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
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
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
