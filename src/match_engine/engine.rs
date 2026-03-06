use anyhow::{Result, anyhow};
use crate::execution::{ExecutionEngine, OrderPreBuilder, PreparedOrders};
use crate::market_data::{MarketUpdate, OrderBook};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

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
    market_update_rx: mpsc::UnboundedReceiver<MarketUpdate>,
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
                "dummy".to_string(),
                Default::default(),
                Default::default(),
            ))),
            goal_orderbook: Arc::new(ArcSwap::from_pointee(OrderBook::new(config.goal_market_id.clone()))),
            match_orderbook: Arc::new(ArcSwap::from_pointee(OrderBook::new(config.clone().match_market_id))),
            order_pre_builder,
            execution_engine,
            market_update_rx,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting match engine for: {}", self.config.name);

        // Start market data handler
        let goal_market_id = self.config.goal_market_id.clone();
        let match_market_id = self.config.match_market_id.clone();
        let goal_orderbook = self.goal_orderbook.clone();
        let match_orderbook = self.match_orderbook.clone();
        let prepared_orders = self.prepared_orders.clone();
        let order_pre_builder = self.order_pre_builder.clone();
        let execution_engine = self.execution_engine.clone();
        let match_id = self.config.id.clone();

        tokio::spawn(async move {
            Self::market_data_handler(
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

    pub fn get_prepared_orders(&self) -> Arc<PreparedOrders> {
        self.prepared_orders.load().clone()
    }

    async fn market_data_handler(
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

        // In a real implementation, this would receive market updates
        // For now, we'll simulate orderbook updates
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Simulate orderbook updates and rebuild prepared orders
            // This would be replaced with actual market data processing
            
            debug!("Updating prepared orders for match: {}", match_id);
            
            // Update execution engine cache
            let current_orders = prepared_orders.load();
            execution_engine.update_prepared_orders(&match_id, Arc::new(current_orders.as_ref().clone()));
        }
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

    pub fn add_match(&self, config: MatchConfig) -> Result<()> {
        let (_market_update_tx, market_update_rx) = tokio::sync::mpsc::unbounded_channel();
        
        let engine = MatchEngine::new(
            config.clone(),
            self.execution_engine.clone(),
            market_update_rx,
        );

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

        let handles: Vec<_> = self.matches
            .iter()
            .map(|entry| {
                let engine = entry.value().clone();
                tokio::spawn(async move {
                    // Note: We can't call start() on Arc<MatchEngine> directly
                    // In a real implementation, we'd handle this differently
                    info!("Match engine would start for: {}", engine.get_config().name);
                })
            })
            .collect();

        futures::future::join_all(handles).await;
        Ok(())
    }
}
