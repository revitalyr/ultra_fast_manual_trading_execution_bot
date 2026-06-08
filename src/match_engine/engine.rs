use crate::execution::{ExecutionEngine, OrderPreBuilder, PreparedOrder, PreparedOrders};
use crate::market_data::{MarketUpdate, OrderBook};
use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub id: String,
    pub name: String,
    pub goal_market_id: String,
    pub match_market_id: String,
    pub max_price_limit: f64,
    pub keyboard_shortcut: Option<char>,
}

impl MatchConfig {
    pub fn new(
        id: String,
        name: String,
        goal_market_id: String,
        match_market_id: String,
        max_price_limit: f64,
        keyboard_shortcut: Option<char>,
    ) -> Self {
        Self {
            id,
            name,
            goal_market_id,
            match_market_id,
            max_price_limit,
            keyboard_shortcut,
        }
    }
}

#[derive(Debug)]
pub struct MatchEngine {
    config: MatchConfig,
    prepared_orders: Arc<ArcSwap<PreparedOrders>>,
    execution_engine: Arc<ExecutionEngine>,
    _market_update_tx: mpsc::UnboundedSender<MarketUpdate>, // Keep sender alive
}

impl MatchEngine {
    pub fn new(
        config: MatchConfig,
        execution_engine: Arc<ExecutionEngine>,
        market_update_rx: mpsc::UnboundedReceiver<MarketUpdate>,
        market_update_tx: mpsc::UnboundedSender<MarketUpdate>,
    ) -> Self {
        info!("Starting match engine for: {}", config.name);
        let order_pre_builder = OrderPreBuilder::new(config.max_price_limit);

        let prepared_orders = Arc::new(ArcSwap::from_pointee(PreparedOrders::new(
            config.id.clone(),
            PreparedOrder::placeholder(),
            PreparedOrder::placeholder(),
        )));
        let goal_orderbook = Arc::new(ArcSwap::from_pointee(OrderBook::new(
            config.goal_market_id.clone(),
        )));
        let match_orderbook = Arc::new(ArcSwap::from_pointee(OrderBook::new(
            config.clone().match_market_id,
        )));

        // Initialize the execution engine's cache with starting orders.
        let initial_orders = prepared_orders.load().clone();
        execution_engine.update_prepared_orders(&config.id, initial_orders);

        // Spawn market data handler with the receiver immediately.
        let h_goal_market_id = config.goal_market_id.clone();
        let h_match_market_id = config.match_market_id.clone();
        let h_goal_orderbook = goal_orderbook.clone();
        let h_match_orderbook = match_orderbook.clone();
        let h_prepared_orders = prepared_orders.clone();
        let h_order_pre_builder = order_pre_builder.clone();
        let h_execution_engine = execution_engine.clone();
        let h_match_id = config.id.clone();

        tokio::spawn(async move {
            Self::market_data_handler(
                market_update_rx,
                h_goal_market_id,
                h_match_market_id,
                h_goal_orderbook,
                h_match_orderbook,
                h_prepared_orders,
                h_order_pre_builder,
                h_execution_engine,
                h_match_id,
            )
            .await;
        });

        Self {
            config: config.clone(),
            prepared_orders,
            execution_engine,
            _market_update_tx: market_update_tx,
        }
    }

    pub async fn execute(&self) -> Result<()> {
        info!("Executing match: {}", self.config.name);
        self.execution_engine.execute_match(&self.config.id).await
    }

    pub fn get_config(&self) -> &MatchConfig {
        &self.config
    }

    #[allow(dead_code)] // This method is intended for external use (e.g., UI displaying prepared orders) but not used in current demo flow.
    pub fn get_prepared_orders(&self) -> Arc<PreparedOrders> {
        self.prepared_orders.load().clone()
    }

    #[allow(clippy::too_many_arguments)]
    async fn market_data_handler(
        mut market_update_rx: mpsc::UnboundedReceiver<MarketUpdate>,
        goal_market_id: String,
        match_market_id: String,
        goal_orderbook: Arc<ArcSwap<OrderBook>>,
        match_orderbook: Arc<ArcSwap<OrderBook>>,
        prepared_orders: Arc<ArcSwap<PreparedOrders>>,
        order_pre_builder: OrderPreBuilder,
        execution_engine: Arc<ExecutionEngine>,
        match_id: String,
    ) {
        info!("Market data handler started for match: {}", match_id);

        while let Some(first) = market_update_rx.recv().await {
            let mut latest_goal: Option<OrderBook> = None;
            let mut latest_match: Option<OrderBook> = None;

            // Apply the first update
            if let crate::market_data::MarketUpdateType::OrderBookUpdate(ob) = first.update_type {
                if ob.market_id == goal_market_id {
                    latest_goal = Some(ob);
                } else if ob.market_id == match_market_id {
                    latest_match = Some(ob);
                }
            }

            // Drain any pending updates in a burst, keeping only the latest state per market
            while let Ok(update) = market_update_rx.try_recv() {
                if let crate::market_data::MarketUpdateType::OrderBookUpdate(ob) =
                    update.update_type
                {
                    if ob.market_id == goal_market_id {
                        goal_orderbook.store(Arc::new(ob));
                    } else if ob.market_id == match_market_id {
                        match_orderbook.store(Arc::new(ob));
                    }
                }
            }

            // Apply the latest snapshot
            if let Some(ob) = latest_goal {
                goal_orderbook.store(Arc::new(ob));
            }
            if let Some(ob) = latest_match {
                match_orderbook.store(Arc::new(ob));
            }

            // Rebuild prepared orders once with the latest state
            let current_goal_ob = goal_orderbook.load();
            let current_match_ob = match_orderbook.load();

            if let Err(e) = order_pre_builder.update_orders_on_market_data(
                &match_id,
                &goal_market_id,
                &match_market_id,
                &current_goal_ob,
                &current_match_ob,
                &prepared_orders,
            ) {
                error!(
                    "Failed to rebuild prepared orders for match {}: {}",
                    match_id, e
                );
            } else {
                let new_prepared_orders = prepared_orders.load();
                execution_engine.update_prepared_orders(&match_id, new_prepared_orders.clone());
            }
        }
        info!("Market data handler stopped for match: {}", match_id);
    }
}

#[derive(Debug)]
pub struct MatchManager {
    matches: DashMap<String, Arc<MatchEngine>>,
    execution_engine: Arc<ExecutionEngine>,
}

impl MatchManager {
    pub fn new(execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            matches: DashMap::new(),
            execution_engine,
        }
    }

    pub fn add_match(&self, config: MatchConfig) {
        let (market_update_tx, market_update_rx) = tokio::sync::mpsc::unbounded_channel();

        let engine = MatchEngine::new(
            config.clone(),
            self.execution_engine.clone(),
            market_update_rx,
            market_update_tx,
        );

        let engine_arc = Arc::new(engine);
        self.matches.insert(config.id.clone(), engine_arc);

        info!("Added match: {} ({})", config.name, config.id);
    }

    pub fn get_match(&self, match_id: &str) -> Option<Arc<MatchEngine>> {
        self.matches.get(match_id).map(|entry| entry.clone())
    }

    pub fn get_all_matches(&self) -> Vec<Arc<MatchEngine>> {
        self.matches.iter().map(|entry| entry.clone()).collect()
    }

    pub async fn execute_match(&self, match_id: &str) -> Result<()> {
        if let Some(engine) = self.get_match(match_id) {
            engine.execute().await
        } else {
            Err(anyhow::anyhow!("Match not found: {}", match_id))
        }
    }

    pub async fn start_all(&self) -> Result<()> {
        info!("Starting all match engines");

        // Engines are now started automatically in add_match
        info!("All {} match engines are active", self.matches.len());
        Ok(())
    }
}
