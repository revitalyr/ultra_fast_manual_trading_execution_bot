use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: u64,
}

impl OrderBook {
    pub fn new(market_id: String) -> Self {
        Self {
            market_id,
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: 0,
        }
    }

    pub fn get_best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    pub fn get_best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }

    pub fn get_spread(&self) -> Option<f64> {
        if let (Some(bid), Some(ask)) = (self.get_best_bid(), self.get_best_ask()) {
            Some(ask.price - bid.price)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    pub market_id: String,
    pub update_type: MarketUpdateType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketUpdateType {
    OrderBookUpdate(OrderBook),
    Trade(Trade),
    LiquidityUpdate(LiquidityUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub price: f64,
    pub size: f64,
    pub side: OrderSide,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityUpdate {
    pub added_liquidity: f64,
    pub removed_liquidity: f64,
    pub total_liquidity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub name: String,
    pub question: String,
    pub outcome_type: OutcomeType,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeType {
    Binary, // YES/NO
    Categorical(Vec<String>), // Multiple outcomes
}

impl Market {
    pub fn new(id: String, name: String, question: String, outcome_type: OutcomeType) -> Self {
        Self {
            id,
            name,
            question,
            outcome_type,
            is_active: true,
        }
    }
}
