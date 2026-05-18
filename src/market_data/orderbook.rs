use serde::{Deserialize, Serialize};

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
    pub updated_at: u64,
}

impl OrderBook {
    pub fn new(market_id: String) -> Self {
        Self {
            market_id,
            bids: Vec::with_capacity(20),
            asks: Vec::with_capacity(20),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Обновляет уровень Bid. Если размер 0 — удаляет уровень.
    /// Bids всегда отсортированы по убыванию цены.
    pub fn update_bid(&mut self, price: f64, size: f64) {
        if let Some(pos) = self.bids.iter().position(|level| (level.price - price).abs() < f64::EPSILON) {
            if size <= 0.0 {
                self.bids.remove(pos);
            } else {
                self.bids[pos].size = size;
            }
        } else if size > 0.0 {
            self.bids.push(PriceLevel { price, size });
            self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        }
        self.touch();
    }

    /// Обновляет уровень Ask. Если размер 0 — удаляет уровень.
    /// Asks всегда отсортированы по возрастанию цены.
    pub fn update_ask(&mut self, price: f64, size: f64) {
        if let Some(pos) = self.asks.iter().position(|level| (level.price - price).abs() < f64::EPSILON) {
            if size <= 0.0 {
                self.asks.remove(pos);
            } else {
                self.asks[pos].size = size;
            }
        } else if size > 0.0 {
            self.asks.push(PriceLevel { price, size });
            self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
        }
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    #[allow(dead_code)] // Used by OrderPreBuilder
    pub fn get_best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    #[allow(dead_code)] // Used by OrderPreBuilder
    pub fn get_best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }

    #[allow(dead_code)] // Not used in current demo, but useful for analysis
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
