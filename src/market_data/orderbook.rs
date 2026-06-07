use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: String,
    pub bids: BTreeMap<OrderedFloat, f64>, // price -> size, sorted descending for bids
    pub asks: BTreeMap<OrderedFloat, f64>, // price -> size, sorted ascending for asks
    pub updated_at: u64,
}

/// Wrapper for f64 to enable BTreeMap ordering with correct IEEE 754 total ordering.
/// Uses bit manipulation to ensure NaN handling and correct ordering for negative values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrderedFloat(u64);

impl OrderedFloat {
    /// Converts f64 to ordered bits. Handles:
    /// - Negative floats: flips sign bit so -inf < ... < -0 < +0 < ... < +inf
    /// - NaN: all NaNs sort at the end (greater than any number)
    fn new(value: f64) -> Self {
        let bits = value.to_bits();
        // Flip sign bit for correct ordering: negative values become larger unsigned
        // This makes -inf (0xFFF...) the largest, +inf (0x7FF...) the smallest positive
        OrderedFloat(if bits & 0x8000_0000_0000_0000 != 0 {
            !bits // Negative: invert all bits
        } else {
            bits | 0x8000_0000_0000_0000 // Positive: set sign bit
        })
    }
}

impl From<OrderedFloat> for f64 {
    fn from(ordered: OrderedFloat) -> Self {
        let bits = ordered.0;
        // Reverse the transformation
        let original_bits = if bits & 0x8000_0000_0000_0000 != 0 {
            bits & 0x7FFF_FFFF_FFFF_FFFF // Positive: clear sign bit
        } else {
            !bits // Negative: invert all bits back
        };
        f64::from_bits(original_bits)
    }
}

impl OrderBook {
    pub fn new(market_id: String) -> Self {
        Self {
            market_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Обновляет уровень Bid. Если размер 0 — удаляет уровень.
    /// Bids отсортированы по убыванию цены (OrderedFloat обеспечивает правильный порядок).
    /// O(log n) операция.
    pub fn update_bid(&mut self, price: f64, size: f64) {
        let key = OrderedFloat::new(price);
        if size <= 0.0 {
            self.bids.remove(&key);
        } else {
            self.bids.insert(key, size);
        }
        self.touch();
    }

    /// Обновляет уровень Ask. Если размер 0 — удаляет уровень.
    /// Asks отсортированы по возрастанию цены.
    /// O(log n) операция.
    pub fn update_ask(&mut self, price: f64, size: f64) {
        let key = OrderedFloat::new(price);
        if size <= 0.0 {
            self.asks.remove(&key);
        } else {
            self.asks.insert(key, size);
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
    pub fn get_best_bid(&self) -> Option<PriceLevel> {
        self.bids
            .first_key_value()
            .map(|(key, &size)| PriceLevel {
                price: (*key).into(),
                size,
            })
    }

    #[allow(dead_code)] // Used by OrderPreBuilder
    pub fn get_best_ask(&self) -> Option<PriceLevel> {
        self.asks
            .first_key_value()
            .map(|(key, &size)| PriceLevel {
                price: (*key).into(),
                size,
            })
    }

    #[allow(dead_code)] // Not used in current demo, but useful for analysis
    pub fn get_spread(&self) -> Option<f64> {
        if let (Some(bid), Some(ask)) = (self.get_best_bid(), self.get_best_ask()) {
            Some(ask.price - bid.price)
        } else {
            None
        }
    }

    /// Convert BTreeMap to Vec<PriceLevel> for serialization/external use
    #[allow(dead_code)]
    pub fn bids_as_vec(&self) -> Vec<PriceLevel> {
        self.bids
            .iter()
            .map(|(key, &size)| PriceLevel {
                price: (*key).into(),
                size,
            })
            .collect()
    }

    /// Convert BTreeMap to Vec<PriceLevel> for serialization/external use
    #[allow(dead_code)]
    pub fn asks_as_vec(&self) -> Vec<PriceLevel> {
        self.asks
            .iter()
            .map(|(key, &size)| PriceLevel {
                price: (*key).into(),
                size,
            })
            .collect()
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
