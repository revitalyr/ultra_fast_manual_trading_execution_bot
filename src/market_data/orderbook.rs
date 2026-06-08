use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const F64_SIGN_BIT: u64 = 0x8000_0000_0000_0000;
const F64_CLEAR_SIGN_BIT: u64 = 0x7FFF_FFFF_FFFF_FFFF;

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
        OrderedFloat(if bits & F64_SIGN_BIT != 0 {
            !bits // Negative: invert all bits
        } else {
            bits | F64_SIGN_BIT // Positive: set sign bit
        })
    }
}

impl From<OrderedFloat> for f64 {
    fn from(ordered: OrderedFloat) -> Self {
        let bits = ordered.0;
        // Reverse the transformation
        let original_bits = if bits & F64_SIGN_BIT != 0 {
            bits & F64_CLEAR_SIGN_BIT // Positive: clear sign bit
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
            updated_at: crate::util::now_millis(),
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
        self.updated_at = crate::util::now_millis();
    }

    #[allow(dead_code)] // Used by OrderPreBuilder
    pub fn get_best_bid(&self) -> Option<PriceLevel> {
        self.bids
            .last_key_value()
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_bid_insert_new() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        let bid = ob.get_best_bid().unwrap();
        assert_eq!(bid.price, 100.0);
        assert_eq!(bid.size, 10.0);
    }

    #[test]
    fn test_update_bid_overwrite_same_price() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        ob.update_bid(100.0, 25.0);
        let bid = ob.get_best_bid().unwrap();
        assert_eq!(bid.price, 100.0);
        assert_eq!(bid.size, 25.0);
        assert_eq!(ob.bids.len(), 1);
    }

    #[test]
    fn test_update_bid_remove_with_zero() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        ob.update_bid(100.0, 0.0);
        assert!(ob.get_best_bid().is_none());
        assert!(ob.bids.is_empty());
    }

    #[test]
    fn test_update_bid_remove_with_negative() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        ob.update_bid(100.0, -1.0);
        assert!(ob.get_best_bid().is_none());
    }

    #[test]
    fn test_update_bid_descending_order() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(99.0, 5.0);
        ob.update_bid(101.0, 8.0);
        ob.update_bid(100.0, 3.0);
        let best = ob.get_best_bid().unwrap();
        assert_eq!(best.price, 101.0);
        assert_eq!(best.size, 8.0);
    }

    #[test]
    fn test_update_bid_multiple_levels() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        ob.update_bid(99.0, 5.0);
        ob.update_bid(98.0, 2.0);
        assert_eq!(ob.bids.len(), 3);
        assert_eq!(ob.get_best_bid().unwrap().price, 100.0);
    }

    #[test]
    fn test_update_ask_insert_new() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(100.0, 10.0);
        let ask = ob.get_best_ask().unwrap();
        assert_eq!(ask.price, 100.0);
        assert_eq!(ask.size, 10.0);
    }

    #[test]
    fn test_update_ask_overwrite_same_price() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(100.0, 10.0);
        ob.update_ask(100.0, 30.0);
        let ask = ob.get_best_ask().unwrap();
        assert_eq!(ask.price, 100.0);
        assert_eq!(ask.size, 30.0);
        assert_eq!(ob.asks.len(), 1);
    }

    #[test]
    fn test_update_ask_remove_with_zero() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(100.0, 10.0);
        ob.update_ask(100.0, 0.0);
        assert!(ob.get_best_ask().is_none());
        assert!(ob.asks.is_empty());
    }

    #[test]
    fn test_update_ask_ascending_order() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(102.0, 5.0);
        ob.update_ask(100.0, 8.0);
        ob.update_ask(101.0, 3.0);
        let best = ob.get_best_ask().unwrap();
        assert_eq!(best.price, 100.0);
        assert_eq!(best.size, 8.0);
    }

    #[test]
    fn test_update_ask_multiple_levels() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(100.0, 10.0);
        ob.update_ask(101.0, 5.0);
        ob.update_ask(102.0, 2.0);
        assert_eq!(ob.asks.len(), 3);
        assert_eq!(ob.get_best_ask().unwrap().price, 100.0);
    }

    #[test]
    fn test_update_bid_updates_timestamp() {
        let mut ob = OrderBook::new("test".to_string());
        let ts = ob.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));
        ob.update_bid(100.0, 10.0);
        assert!(ob.updated_at > ts);
    }

    #[test]
    fn test_update_ask_updates_timestamp() {
        let mut ob = OrderBook::new("test".to_string());
        let ts = ob.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));
        ob.update_ask(100.0, 10.0);
        assert!(ob.updated_at > ts);
    }

    #[test]
    fn test_bid_ask_independent() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 10.0);
        ob.update_ask(101.0, 5.0);
        assert_eq!(ob.bids.len(), 1);
        assert_eq!(ob.asks.len(), 1);
        assert_eq!(ob.get_best_bid().unwrap().price, 100.0);
        assert_eq!(ob.get_best_ask().unwrap().price, 101.0);
    }

    #[test]
    fn test_update_bid_remove_nonexistent() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(100.0, 0.0);
        assert!(ob.bids.is_empty());
    }

    #[test]
    fn test_update_ask_remove_nonexistent() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_ask(100.0, 0.0);
        assert!(ob.asks.is_empty());
    }

    #[test]
    fn test_negative_zero_bid_price() {
        let mut ob = OrderBook::new("test".to_string());
        ob.update_bid(-0.0, 10.0);
        ob.update_bid(0.0, 5.0);
        // -0.0 sorts below 0.0 in IEEE 754 total order after OrderedFloat transformation
        assert_eq!(ob.bids.len(), 2);
    }
}
