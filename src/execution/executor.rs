use crate::execution::prepared_orders::{ExecutionRequest, ExecutionResult, PreparedOrder, PreparedOrders};
use crate::trading::polymarket_client::PolymarketClient;
use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug)]
pub struct ExecutionEngine {
    trading_client: Arc<PolymarketClient>,
    prepared_orders_cache: Arc<DashMap<String, Arc<ArcSwap<PreparedOrders>>>>,
    execution_tx: mpsc::UnboundedSender<ExecutionRequest>,
}

impl ExecutionEngine {
    pub fn new(trading_client: Arc<PolymarketClient>) -> Self {
        let (execution_tx, execution_rx) = mpsc::unbounded_channel();
        let prepared_orders_cache = Arc::new(DashMap::new());
        
        let engine = Self {
            trading_client,
            prepared_orders_cache,
            execution_tx,
        };

        // Start execution handler
        let client_clone = engine.trading_client.clone();
        let cache_clone = engine.prepared_orders_cache.clone();
        tokio::spawn(async move {
            Self::execution_handler(execution_rx, client_clone, cache_clone).await;
        });

        engine
    }

    #[allow(dead_code)] // Used in examples/demo.rs, but not directly in lib.rs or main.rs
    pub fn get_execution_sender(&self) -> mpsc::UnboundedSender<ExecutionRequest> {
        self.execution_tx.clone()
    }

    pub fn update_prepared_orders(&self, match_id: &str, orders: Arc<crate::execution::prepared_orders::PreparedOrders>) {
        if let Some(swap) = self.prepared_orders_cache.get(match_id) {
            // Hot path: update existing atomic pointer
            swap.store(orders);
        } else {
            // Cold path: initialize cache entry
            self.prepared_orders_cache.insert(match_id.to_string(), Arc::new(ArcSwap::new(orders)));
        }
    }

    pub async fn execute_match(&self, match_id: &str) -> Result<()> {
        if let Some(swap_arc) = self.prepared_orders_cache.get(match_id) {
            let orders = swap_arc.load();
            let request = ExecutionRequest::new(match_id.to_string(), Arc::new(orders.as_ref().clone()));
            
            if let Err(e) = self.execution_tx.send(request) {
                error!("Failed to send execution request: {}", e);
                return Err(anyhow::anyhow!("Failed to queue execution"));
            }
            
            info!("Execution request queued for match: {}", match_id);
        } else {
            error!("No prepared orders found for match: {}", match_id);
            return Err(anyhow::anyhow!("No prepared orders available"));
        }
        
        Ok(())
    }

    async fn execution_handler(
        mut execution_rx: mpsc::UnboundedReceiver<ExecutionRequest>,
        trading_client: Arc<PolymarketClient>,
        _prepared_orders_cache: Arc<DashMap<String, Arc<ArcSwap<PreparedOrders>>>>,
    ) {
        info!("Execution handler started");

        while let Some(request) = execution_rx.recv().await {
            let start_time = Instant::now();
            
            info!("Executing match: {}", request.match_id);

            // Execute both orders in parallel for ultra-low latency
            let goal_order = request.orders.goal_market_order.clone();
            let match_order = request.orders.match_result_order.clone();
            let client = trading_client.clone();

            let (goal_result, match_result) = tokio::join!(
                Self::execute_single_order(client.clone(), goal_order),
                Self::execute_single_order(client.clone(), match_order)
            );

            let execution_time = start_time.elapsed().as_millis() as u64;

            info!(
                "Execution completed for match {} in {}ms. Goal: {}, Match: {}",
                request.match_id,
                execution_time,
                if goal_result.is_ok() { "SUCCESS" } else { "FAILED" },
                if match_result.is_ok() { "SUCCESS" } else { "FAILED" }
            );

            // Log results for monitoring
            if let Err(e) = &goal_result {
                error!("Goal order execution failed: {}", e);
            }
            if let Err(e) = &match_result {
                error!("Match order execution failed: {}", e);
            }
        }

        info!("Execution handler stopped");
    }

    async fn execute_single_order(
        client: Arc<PolymarketClient>,
        order: PreparedOrder,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        match client.submit_order(&order).await {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                Ok(ExecutionResult {
                    order_id: order.id,
                    success: true,
                    error: None,
                    execution_time_ms: execution_time,
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                Ok(ExecutionResult {
                    order_id: order.id,
                    success: false,
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                })
            }
        }
    }
}
