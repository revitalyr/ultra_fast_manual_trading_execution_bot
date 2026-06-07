use anyhow::Result;
use crate::execution::{ExecutionEngine, OrderPreBuilder, PreparedOrders};
use crate::market_data::{MarketUpdate, OrderBook};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;use tracing::{debug, error, info, warn};

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
    goal_orderbook: Arc<ArcSwap<OrderBook>>,
    match_orderbook: Arc<ArcSwap<OrderBook>>,
    order_pre_builder: OrderPreBuilder,
    execution_engine: Arc<ExecutionEngine>,
    market_update_rx: Option<mpsc::UnboundedReceiver<MarketUpdate>>,
}

impl MatchEngine {
    pub fn new(
        config: MatchConfig,
        execution_engine: Arc<ExecutionEngine>,
        market_update_rx: mpsc::UnboundedReceiver<MarketUpdate>,
    ) -> Self {
        let order_pre_builder = OrderPreBuilder::new(config.max_price_limit);
        
        Self {
            config: config.clone(),
            prepared_orders: Arc::new(ArcSwap::from_pointee(PreparedOrders::new(
                config.id.clone(),
                Default::default(),
                Default::default(),
            ))),
            goal_orderbook: Arc::new(ArcSwap::from_pointee(OrderBook::new(config.goal_market_id.clone()))),
            match_orderbook: Arc::new(ArcSwap::from_pointee(OrderBook::new(config.clone().match_market_id))),
            order_pre_builder,
            execution_engine,
            market_update_rx: Some(market_update_rx),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting match engine for: {}", self.config.name);

        // Start market data handler
        let goal_market_id = self.config.goal_market_id.clone(); // Clone for the handler
        let match_market_id = self.config.match_market_id.clone();
        let goal_orderbook = self.goal_orderbook.clone();
        let match_orderbook = self.match_orderbook.clone();
        let prepared_orders = self.prepared_orders.clone();
        let order_pre_builder = self.order_pre_builder.clone();
        let execution_engine = self.execution_engine.clone();
        let match_id = self.config.id.clone();

        let market_update_rx = self.market_update_rx.take().expect("Market update receiver already taken or not set");
        
        // Initialize the execution engine's cache with our starting orders.
        // This ensures the match is ready to be triggered even before the first market data update arrives,
        // preventing "No prepared orders available" errors.
        let initial_orders = self.prepared_orders.load().clone();
        self.execution_engine.update_prepared_orders(&self.config.id, initial_orders);

        tokio::spawn(async move {
            Self::market_data_handler(
                market_update_rx,
                goal_market_id,
                match_market_id,
                goal_orderbook,
                match_orderbook,
                prepared_orders,
                order_pre_builder,
                execution_engine,
                match_id,
            ).await;
        });

        Ok(())
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
        
        while let Some(update) = market_update_rx.recv().await {
            debug!("Received market update for market: {}", update.market_id);

            match update.update_type {
                crate::market_data::MarketUpdateType::OrderBookUpdate(ob_update) => {
                    if ob_update.market_id == goal_market_id {
                        goal_orderbook.store(Arc::new(ob_update));
                        debug!("Goal orderbook updated for match: {}", match_id);
                    } else if ob_update.market_id == match_market_id {
                        match_orderbook.store(Arc::new(ob_update));
                        debug!("Match orderbook updated for match: {}", match_id);
                    } else {
                        warn!("Received orderbook update for unknown market_id: {}", ob_update.market_id);
                        continue;
                    }

                    // After updating an orderbook, rebuild prepared orders
                    let current_goal_ob = goal_orderbook.load();
                    let current_match_ob = match_orderbook.load();

                    if let Err(e) = order_pre_builder.update_orders_on_market_data(
                        &match_id, &goal_market_id, &match_market_id, &current_goal_ob, &current_match_ob, &prepared_orders,
                    ) {
                        error!("Failed to rebuild prepared orders for match {}: {}", match_id, e);
                    } else {
                        // Update execution engine cache with the newly prepared orders
                        let new_prepared_orders = prepared_orders.load();
                        execution_engine.update_prepared_orders(&match_id, new_prepared_orders.clone());
                    }
                },
                _ => {
                    debug!("Unhandled market update type for market: {}", update.market_id);
                }
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

    pub async fn add_match(&self, config: MatchConfig) -> Result<()> {
        let (_market_update_tx, market_update_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create the MatchEngine
        let mut engine = MatchEngine::new(
            config.clone(),
            self.execution_engine.clone(),
            market_update_rx,
        );

        engine.start().await?;

        let engine_arc = Arc::new(engine);
        self.matches.insert(config.id.clone(), engine_arc);

        info!("Added match: {} ({})", config.name, config.id);
        Ok(())
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
