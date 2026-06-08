use crate::execution::prepared_orders::{PreparedOrder, PreparedOrders};
use crate::market_data::{OrderBook, OrderSide, OrderType};
use anyhow::Result;
use arc_swap::ArcSwap;
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

        let prepared_orders = PreparedOrders::new(match_id.to_string(), goal_order, match_order);

        info!("Built prepared orders for match: {}", match_id);
        debug!(
            "Goal order size: {}, Match order price: {:?}",
            prepared_orders.goal_market_order.size, prepared_orders.match_result_order.price
        );

        Ok(prepared_orders)
    }

    fn build_goal_market_order(
        &self,
        market_id: &str,
        orderbook: &OrderBook,
    ) -> Result<PreparedOrder> {
        // Use get_best_ask to find the best price and its size
        let best_ask = orderbook
            .get_best_ask()
            .ok_or_else(|| anyhow::anyhow!("No ask liquidity available in goal market"))?;

        // For a market order, we might want to consume all available liquidity up to a certain point
        let total_liquidity = best_ask.size; // Simplified: just take the best ask's size for now

        if total_liquidity <= 0.0 {
            return Err(anyhow::anyhow!("No liquidity available in goal market"));
        }

        let order = PreparedOrder::with_params(
            market_id.to_string(),
            OrderType::Market,
            OrderSide::Buy,
            total_liquidity,
            None,
        );

        Ok(order)
    }

    fn build_match_result_order(
        &self,
        market_id: &str,
        orderbook: &OrderBook,
    ) -> Result<PreparedOrder> {
        // Find the best ask price within our limit
        let best_ask_level = orderbook
            .get_best_ask()
            .ok_or_else(|| anyhow::anyhow!("No orders available within price limit"))?;

        if best_ask_level.price > self.max_price_limit {
            return Err(anyhow::anyhow!("Best ask price is above max price limit"));
        }

        let best_price = best_ask_level.price;
        let order_size = best_ask_level.size; // Simplified: just take the best ask's size for now

        if order_size <= 0.0 {
            return Err(anyhow::anyhow!("No liquidity available within price limit"));
        }

        let order = PreparedOrder::with_params(
            market_id.to_string(),
            OrderType::Limit { price: best_price },
            OrderSide::Buy,
            order_size,
            Some(best_price),
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
        match self.build_orders_for_match(
            match_id,
            goal_market_id,
            match_market_id,
            goal_orderbook,
            match_orderbook,
        ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn orderbook_with_asks(pairs: &[(f64, f64)]) -> OrderBook {
        let mut ob = OrderBook::new("test".to_string());
        for &(price, size) in pairs {
            ob.update_ask(price, size);
        }
        ob
    }

    fn orderbook_with_bids(pairs: &[(f64, f64)]) -> OrderBook {
        let mut ob = OrderBook::new("test".to_string());
        for &(price, size) in pairs {
            ob.update_bid(price, size);
        }
        ob
    }

    #[test]
    fn test_build_orders_success() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_ok());

        let orders = result.unwrap();
        assert_eq!(orders.match_id, "m1");
        assert_eq!(orders.goal_market_order.market_id, "goal");
        assert_eq!(orders.match_result_order.market_id, "match");
    }

    #[test]
    fn test_build_orders_goal_market_order_type() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let orders = builder
            .build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob)
            .unwrap();

        match orders.goal_market_order.order_type {
            OrderType::Market => {}
            _ => panic!("Goal order should be Market type"),
        }
        assert_eq!(orders.goal_market_order.side, OrderSide::Buy);
        assert_eq!(orders.goal_market_order.size, 100.0);
        assert!(orders.goal_market_order.price.is_none());
    }

    #[test]
    fn test_build_orders_match_order_type() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let orders = builder
            .build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob)
            .unwrap();

        match orders.match_result_order.order_type {
            OrderType::Limit { price } => assert_eq!(price, 0.85),
            _ => panic!("Match order should be Limit type"),
        }
        assert_eq!(orders.match_result_order.side, OrderSide::Buy);
        assert_eq!(orders.match_result_order.size, 50.0);
        assert_eq!(orders.match_result_order.price, Some(0.85));
    }

    #[test]
    fn test_build_orders_goal_no_liquidity() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = OrderBook::new("goal".to_string());
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No ask liquidity"));
    }

    #[test]
    fn test_build_orders_match_no_liquidity() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = OrderBook::new("match".to_string());

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No orders available"));
    }

    #[test]
    fn test_build_orders_price_exceeds_limit() {
        let builder = OrderPreBuilder::new(0.80);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("above max price limit"));
    }

    #[test]
    fn test_build_orders_goal_zero_size() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 0.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_orders_match_zero_size() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 0.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_orders_picks_lowest_goal_ask() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(1.10, 200.0), (0.90, 100.0), (0.95, 50.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let orders = builder
            .build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob)
            .unwrap();
        // Should pick the lowest ask (0.90) for the goal market order
        assert_eq!(orders.goal_market_order.size, 100.0);
    }

    #[test]
    fn test_build_orders_picks_lowest_match_ask() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.90, 30.0), (0.80, 50.0), (0.85, 20.0)]);

        let orders = builder
            .build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob)
            .unwrap();
        // Should pick the lowest ask (0.80) for the match limit order
        assert_eq!(orders.match_result_order.price, Some(0.80));
        assert_eq!(orders.match_result_order.size, 50.0);
    }

    #[test]
    fn test_build_orders_at_price_limit_boundary() {
        let builder = OrderPreBuilder::new(0.85);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().match_result_order.price, Some(0.85));
    }

    #[test]
    fn test_update_orders_on_market_data_success() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, 100.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);
        let cache = ArcSwap::new(Arc::new(PreparedOrders::new(
            "init".to_string(),
            PreparedOrder::placeholder(),
            PreparedOrder::placeholder(),
        )));

        let result = builder
            .update_orders_on_market_data("m1", "goal", "match", &goal_ob, &match_ob, &cache);
        assert!(result.is_ok());

        let stored = cache.load();
        assert_eq!(stored.match_id, "m1");
        assert_eq!(stored.goal_market_order.size, 100.0);
        assert_eq!(stored.match_result_order.size, 50.0);
    }

    #[test]
    fn test_update_orders_on_market_data_error_keeps_old_cache() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = OrderBook::new("goal".to_string());
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);
        let old_orders = Arc::new(PreparedOrders::new(
            "old".to_string(),
            PreparedOrder::placeholder(),
            PreparedOrder::placeholder(),
        ));
        let cache = ArcSwap::new(old_orders.clone());

        let result = builder
            .update_orders_on_market_data("m1", "goal", "match", &goal_ob, &match_ob, &cache);
        assert!(result.is_err());

        let stored = cache.load();
        assert_eq!(stored.match_id, "old");
    }

    #[test]
    fn test_build_orders_negative_goal_liquidity() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_asks(&[(0.95, -10.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_orders_bids_ignored_in_goal() {
        let builder = OrderPreBuilder::new(1.0);
        let goal_ob = orderbook_with_bids(&[(0.90, 999.0)]);
        let match_ob = orderbook_with_asks(&[(0.85, 50.0)]);

        let result = builder.build_orders_for_match("m1", "goal", "match", &goal_ob, &match_ob);
        // Bids don't count as liquidity for asks
        assert!(result.is_err());
    }

    #[test]
    fn test_new_sets_max_price_limit() {
        let builder = OrderPreBuilder::new(0.75);
        assert_eq!(builder.max_price_limit, 0.75);
    }
}
