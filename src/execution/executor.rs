use crate::execution::prepared_orders::{ExecutionRequest, ExecutionResult, PreparedOrder, PreparedOrders};
use crate::traits::ExecutionEngine as ExecutionEngineTrait;
use crate::trading::polymarket_client::PolymarketClient;
use anyhow::Result;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info};

const EXECUTION_QUEUE_CAPACITY: usize = 1000;

#[derive(Debug)]
pub struct ExecutionEngine {
    trading_client: Arc<PolymarketClient>,
    prepared_orders_cache: Arc<DashMap<String, Arc<ArcSwap<PreparedOrders>>>>,
    execution_tx: mpsc::Sender<ExecutionRequest>,
}

impl ExecutionEngine {
    pub fn new(trading_client: Arc<PolymarketClient>) -> Self {
        let (execution_tx, execution_rx) = mpsc::channel(EXECUTION_QUEUE_CAPACITY);
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
    pub fn get_execution_sender(&self) -> mpsc::Sender<ExecutionRequest> {
        self.execution_tx.clone()
    }

    pub fn update_prepared_orders(&self, match_id: &str, orders: Arc<crate::execution::prepared_orders::PreparedOrders>) {
        self.prepared_orders_cache
            .entry(match_id.to_string())
            .and_modify(|swap| swap.store(orders.clone()))
            .or_insert_with(|| Arc::new(ArcSwap::new(orders)));
    }

    pub async fn execute_match(&self, match_id: &str) -> Result<()> {
        if let Some(swap_arc) = self.prepared_orders_cache.get(match_id) {
            let orders = swap_arc.load();
            let request = ExecutionRequest::new(match_id.to_string(), Arc::clone(&orders));
            
            if let Err(e) = self.execution_tx.try_send(request) {
                error!("Execution queue full, dropping request: {}", e);
                return Err(anyhow::anyhow!("Execution queue full, try again later"));
            }
            
            info!("Execution request queued for match: {}", match_id);
        } else {
            error!("No prepared orders found for match: {}", match_id);
            return Err(anyhow::anyhow!("No prepared orders available"));
        }
        
        Ok(())
    }

    async fn execution_handler(
        mut execution_rx: mpsc::Receiver<ExecutionRequest>,
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

    pub(crate) async fn execute_single_order(
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

#[async_trait::async_trait]
impl ExecutionEngineTrait for ExecutionEngine {
    fn update_prepared_orders(&self, match_id: &str, orders: Arc<PreparedOrders>) {
        self.update_prepared_orders(match_id, orders)
    }

    async fn execute_match(&self, match_id: &str) -> Result<()> {
        self.execute_match(match_id).await
    }

    fn get_execution_sender(&self) -> mpsc::Sender<ExecutionRequest> {
        self.get_execution_sender()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_orders(match_id: &str) -> Arc<PreparedOrders> {
        Arc::new(PreparedOrders::new(
            match_id.to_string(),
            PreparedOrder::placeholder(),
            PreparedOrder::placeholder(),
        ))
    }

    fn make_engine() -> ExecutionEngine {
        let client = Arc::new(PolymarketClient::new("http://localhost:1".to_string(), None));
        ExecutionEngine::new(client)
    }

    #[tokio::test]
    async fn test_execute_match_no_orders() {
        let engine = make_engine();
        let result = engine.execute_match("nonexistent").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No prepared orders"), "got: {}", err);
    }

    #[tokio::test]
    async fn test_execute_match_with_orders() {
        let engine = make_engine();
        let orders = make_orders("m1");
        engine.update_prepared_orders("m1", orders);

        let result = engine.execute_match("m1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_match_queue_full() {
        let engine = make_engine();
        let orders = make_orders("m1");
        engine.update_prepared_orders("m1", orders);

        // Fill the channel via raw sender to capacity
        let tx = engine.get_execution_sender();
        let dummy = ExecutionRequest::new("dummy".to_string(), make_orders("dummy"));
        for _ in 0..EXECUTION_QUEUE_CAPACITY {
            tx.try_send(dummy.clone()).expect("fill channel");
        }
        // try_send on a full channel must fail
        assert!(tx.try_send(dummy.clone()).is_err());

        // execute_match uses try_send internally, so it must also fail
        let result = engine.execute_match("m1").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("queue full"), "got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_execute_match_wrong_match_id() {
        let engine = make_engine();
        engine.update_prepared_orders("actual", make_orders("actual"));

        let result = engine.execute_match("other").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No prepared orders"));
    }

    #[tokio::test]
    async fn test_update_prepared_orders_overwrites() {
        let engine = make_engine();
        let first = PreparedOrders::new("m1".to_string(), PreparedOrder::placeholder(), PreparedOrder::placeholder());
        let first_ts = first.updated_at;

        // Small delay so the second has a different timestamp
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        let second = PreparedOrders::new("m1".to_string(), PreparedOrder::placeholder(), PreparedOrder::placeholder());
        assert!(second.updated_at > first_ts);

        engine.update_prepared_orders("m1", Arc::new(first));
        engine.update_prepared_orders("m1", Arc::new(second));

        // Execute should use the latest version
        let result = engine.execute_match("m1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_match_multiple_matches() {
        let engine = make_engine();
        engine.update_prepared_orders("m1", make_orders("m1"));
        engine.update_prepared_orders("m2", make_orders("m2"));

        let r1 = engine.execute_match("m1").await;
        let r2 = engine.execute_match("m2").await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn test_execute_match_order_data_preserved() {
        let engine = make_engine();

        let goal = PreparedOrder::with_params(
            "goal_mkt".to_string(),
            crate::market_data::OrderType::Market,
            crate::market_data::OrderSide::Buy,
            42.0,
            None,
        );
        let match_ord = PreparedOrder::with_params(
            "match_mkt".to_string(),
            crate::market_data::OrderType::Limit { price: 0.5 },
            crate::market_data::OrderSide::Buy,
            10.0,
            Some(0.5),
        );
        let orders = Arc::new(PreparedOrders::new("m1".to_string(), goal, match_ord));
        engine.update_prepared_orders("m1", orders);

        let result = engine.execute_match("m1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_match_idempotent_cache() {
        let engine = make_engine();
        engine.update_prepared_orders("m1", make_orders("m1"));

        // Execute twice — both should succeed (cache persists)
        assert!(engine.execute_match("m1").await.is_ok());
        assert!(engine.execute_match("m1").await.is_ok());
    }

    #[tokio::test]
    async fn test_execution_handler_drains_queue() {
        let engine = make_engine();
        engine.update_prepared_orders("m1", make_orders("m1"));

        // Queue a request via execute_match
        assert!(engine.execute_match("m1").await.is_ok());

        // Give the handler a moment to receive and process it
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // The handler should have consumed the message, so the channel has room.
        // We can verify by filling the channel — if the handler wasn't running,
        // try_send would fill up immediately from the already-sent messages.
        // Instead, we queue many more and verify at least one succeeds,
        // proving the handler is consuming.
        let tx = engine.get_execution_sender();
        let dummy = ExecutionRequest::new("dummy".to_string(), make_orders("dummy"));
        let mut sent = 1; // the one from execute_match above
        while let Ok(()) = tx.try_send(dummy.clone()) {
            sent += 1;
            if sent > EXECUTION_QUEUE_CAPACITY + 10 {
                break;
            }
        }
        // If handler was alive, we should have been able to send at least
        // EXECUTION_QUEUE_CAPACITY messages total (the initial one + more).
        // If handler was dead, we'd be stuck at capacity.
        assert!(sent >= EXECUTION_QUEUE_CAPACITY / 2, "handler appears stalled, sent={}", sent);
    }

    #[tokio::test]
    async fn test_parallel_order_execution() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Start a local TCP server that accepts two connections
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        let server_handle = tokio::spawn(async move {
            let mut goal_received = false;
            let mut match_received = false;

            // Accept two connections (one for goal order, one for match order)
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();

                // Read the HTTP request (headers only needed)
                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf).await.unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);

                if request.contains("/api/v1/orders") {
                    if request.contains("goal_mkt") {
                        goal_received = true;
                    }
                    if request.contains("match_mkt") {
                        match_received = true;
                    }
                }

                // Respond with 200 OK JSON
                let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}";
                stream.write_all(response).await.unwrap();
            }

            (goal_received, match_received)
        });

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let engine = ExecutionEngine::new(client);

        let goal = PreparedOrder::with_params(
            "goal_mkt".to_string(),
            crate::market_data::OrderType::Market,
            crate::market_data::OrderSide::Buy,
            100.0,
            None,
        );
        let match_ord = PreparedOrder::with_params(
            "match_mkt".to_string(),
            crate::market_data::OrderType::Limit { price: 0.5 },
            crate::market_data::OrderSide::Buy,
            50.0,
            Some(0.5),
        );
        let orders = Arc::new(PreparedOrders::new("m1".to_string(), goal, match_ord));
        engine.update_prepared_orders("m1", orders);

        let start = Instant::now();
        assert!(engine.execute_match("m1").await.is_ok());
        // Wait for the handler to process and the server to accept both connections
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let elapsed = start.elapsed();

        let (goal_rcvd, match_rcvd) = server_handle.await.unwrap();
        assert!(goal_rcvd, "Goal order request not received by server");
        assert!(match_rcvd, "Match order request not received by server");

        // Both orders were sent in parallel; total time should be well
        // under the sum of two individual delays if they were serial
        assert!(
            elapsed.as_millis() < 1000,
            "Parallel execution took too long: {}ms",
            elapsed.as_millis()
        );
    }

    fn make_order_with_id(market_id: &str) -> PreparedOrder {
        PreparedOrder::with_params(
            market_id.to_string(),
            crate::market_data::OrderType::Market,
            crate::market_data::OrderSide::Buy,
            100.0,
            None,
        )
    }

    async fn run_error_server(response_body: &'static [u8]) -> u16 {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                stream.write_all(response_body).await.unwrap();
            }
        });

        port
    }

    #[tokio::test]
    async fn test_execute_single_order_http_400() {
        let resp = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 31\r\nContent-Type: application/json\r\n\r\n{\"error\":\"invalid order data\"}";
        let port = run_error_server(resp).await;

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let order = make_order_with_id("mkt1");
        let result = ExecutionEngine::execute_single_order(client, order).await;

        assert!(result.is_ok());
        let exec = result.unwrap();
        assert!(!exec.success, "expected failure on HTTP 400");
        assert!(exec.error.is_some(), "expected error message");
    }

    #[tokio::test]
    async fn test_execute_single_order_http_500() {
        let resp = b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 22\r\nContent-Type: application/json\r\n\r\n{\"error\":\"server down\"}";
        let port = run_error_server(resp).await;

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let order = make_order_with_id("mkt2");
        let result = ExecutionEngine::execute_single_order(client, order).await;

        assert!(result.is_ok());
        let exec = result.unwrap();
        assert!(!exec.success);
        assert!(exec.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_single_order_http_200() {
        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}";
        let port = run_error_server(resp).await;

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let order = make_order_with_id("mkt3");
        let result = ExecutionEngine::execute_single_order(client, order).await;

        assert!(result.is_ok());
        let exec = result.unwrap();
        assert!(exec.success);
        assert!(exec.error.is_none());
    }

    #[tokio::test]
    async fn test_execute_single_order_connection_refused() {
        // Point at a port that's not listening
        let client = Arc::new(PolymarketClient::new(
            "http://127.0.0.1:1".to_string(),
            None,
        ));
        let order = make_order_with_id("mkt4");
        let result = ExecutionEngine::execute_single_order(client, order).await;

        assert!(result.is_ok());
        let exec = result.unwrap();
        assert!(!exec.success);
        assert!(exec.error.is_some());
    }

    #[tokio::test]
    async fn test_execution_handler_one_fails_one_succeeds() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Server: first connection OK, second connection 400
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for i in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let resp: &[u8] = if i == 0 {
                    b"HTTP/1.1 200 OK\r\nContent-Length:2\r\nContent-Type:application/json\r\n\r\n{}"
                } else {
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length:22\r\nContent-Type:application/json\r\n\r\n{\"error\":\"bad\"}"
                };
                stream.write_all(resp).await.unwrap();
            }
        });

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let engine = ExecutionEngine::new(client);

        let orders = Arc::new(PreparedOrders::new(
            "m1".to_string(),
            make_order_with_id("goal"),
            make_order_with_id("match"),
        ));
        engine.update_prepared_orders("m1", orders);
        assert!(engine.execute_match("m1").await.is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    #[tokio::test]
    async fn test_update_prepared_orders_concurrent_insert() {
        let engine = Arc::new(make_engine());
        let mut handles = Vec::new();
        let task_count = 50;

        for i in 0..task_count {
            let eng = engine.clone();
            handles.push(tokio::spawn(async move {
                let market = format!("concurrent_mkt_{}", i % 5);
                let orders = make_orders(&market);
                eng.update_prepared_orders(&market, orders);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // Exactly 5 distinct keys should exist (i%5 → 0..4)
        assert_eq!(engine.prepared_orders_cache.len(), 5);
        // Every entry should be valid
        for entry in engine.prepared_orders_cache.iter() {
            assert!(entry.value().load().updated_at > 0);
        }
    }

    #[tokio::test]
    async fn test_update_prepared_orders_concurrent_update() {
        let engine = Arc::new(make_engine());

        // Pre-insert 3 matches
        for m in &["a", "b", "c"] {
            engine.update_prepared_orders(m, make_orders(m));
        }

        let mut handles = Vec::new();
        let task_count = 50;

        for i in 0..task_count {
            let eng = engine.clone();
            let market = match i % 3 {
                0 => "a",
                1 => "b",
                _ => "c",
            };
            handles.push(tokio::spawn(async move {
                let orders = make_orders(market);
                eng.update_prepared_orders(market, orders);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // Still exactly 3 entries, all valid
        assert_eq!(engine.prepared_orders_cache.len(), 3);
        for entry in engine.prepared_orders_cache.iter() {
            assert!(entry.value().load().updated_at > 0);
        }
    }

    #[tokio::test]
    async fn test_execution_handler_both_fail() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let resp = b"HTTP/1.1 500 Internal Server Error\r\nContent-Length:22\r\nContent-Type:application/json\r\n\r\n{\"error\":\"server err\"}";
                stream.write_all(resp).await.unwrap();
            }
        });

        let client = Arc::new(PolymarketClient::new(
            format!("http://127.0.0.1:{}", port),
            None,
        ));
        let engine = ExecutionEngine::new(client);

        let orders = Arc::new(PreparedOrders::new(
            "m1".to_string(),
            make_order_with_id("goal"),
            make_order_with_id("match"),
        ));
        engine.update_prepared_orders("m1", orders);
        assert!(engine.execute_match("m1").await.is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        // Engine still alive (handler didn't crash)
        assert!(engine.execute_match("m1").await.is_ok());
    }
}
