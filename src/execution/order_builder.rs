use crate::execution::prepared_orders::{PreparedOrder, PreparedOrders};
use crate::market_data::{OrderBook, OrderSide, OrderType};
use anyhow::Result;
use arc_swap::ArcSwap;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct OrderPreBuilder {
    max_price_limit: f64,
}

impl OrderPreBuilder {
    pub fn new(max_price_limit: f64) -> Self {
        Self { max_price_limit }
    }

    pub fn build_orders_for_match(
        &self,
        match_id: &str,
        goal_market_id: &str,
        match_market_id: &str,
        goal_orderbook: &OrderBook,
        match_orderbook: &OrderBook,
    ) -> Result<PreparedOrders> {
        // Build goal market order - market order to buy all available liquidity
        let goal_order = self.build_goal_market_order(goal_market_id, goal_orderbook)?;

        // Build match result market order - limit order up to max price
        let match_order = self.build_match_result_order(match_market_id, match_orderbook)?;

        let prepared_orders = PreparedOrders::new(
            match_id.to_string(),
            goal_order,
            match_order,
        );

        info!("Built prepared orders for match: {}", match_id);
        debug!("Goal order size: {}, Match order price: {:?}", 
               prepared_orders.goal_market_order.size,
               prepared_orders.match_result_order.price);

        Ok(prepared_orders)
    }

    fn build_goal_market_order(&self, market_id: &str, orderbook: &OrderBook) -> Result<PreparedOrder> {
        // Calculate total available liquidity on the ask side
        let total_liquidity: f64 = orderbook.asks.iter().map(|level| level.size).sum();
        
        if total_liquidity <= 0.0 {
            return Err(anyhow::anyhow!("No liquidity available in goal market"));
        }

        // Build market order payload
        let payload = json!({
            "marketId": market_id,
            "type": "market",
            "side": "buy",
            "size": total_liquidity,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        });

        let order = PreparedOrder::new(
            market_id.to_string(),
            OrderType::Market,
            OrderSide::Buy,
            total_liquidity,
            None,
            bytes::Bytes::from(payload.to_string()),
        );

        Ok(order)
    }

    fn build_match_result_order(&self, market_id: &str, orderbook: &OrderBook) -> Result<PreparedOrder> {
        // Find the best price within our limit
        let best_price = orderbook
            .asks
            .iter()
            .filter(|level| level.price <= self.max_price_limit)
            .map(|level| level.price)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No orders available within price limit"))?;

        let order_size = orderbook
            .asks
            .iter()
            .filter(|level| level.price <= self.max_price_limit)
            .map(|level| level.size)
            .sum();

        if order_size <= 0.0 {
            return Err(anyhow::anyhow!("No liquidity available within price limit"));
        }

        // Build limit order payload
        let payload = json!({
            "marketId": market_id,
            "type": "limit",
            "side": "buy",
            "size": order_size,
            "price": best_price,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        });

        let order = PreparedOrder::new(
            market_id.to_string(),
            OrderType::Limit { price: best_price },
            OrderSide::Buy,
            order_size,
            Some(best_price),
            bytes::Bytes::from(payload.to_string()),
        );

        Ok(order)
    }

    pub fn update_orders_on_market_data(
        &self,
        match_id: &str,
        goal_market_id: &str,
        match_market_id: &str,
        goal_orderbook: &OrderBook,
        match_orderbook: &OrderBook,
        orders_cache: &ArcSwap<PreparedOrders>,
    ) -> Result<()> {
        match self.build_orders_for_match(match_id, goal_market_id, match_market_id, goal_orderbook, match_orderbook) {
            Ok(new_orders) => {
                orders_cache.store(Arc::new(new_orders));
                debug!("Updated prepared orders for match: {}", match_id);
                Ok(())
            }
            Err(e) => {
                debug!("Failed to update orders for match {}: {}", match_id, e);
                Err(e)
            }
        }
    }
}
