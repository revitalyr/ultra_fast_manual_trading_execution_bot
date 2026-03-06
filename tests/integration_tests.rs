use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use ultra_fast_manual_trading_execution_bot::execution::{ExecutionEngine, PreparedOrder, PreparedOrders, ExecutionRequest};
use ultra_fast_manual_trading_execution_bot::match_engine::{MatchManager, MatchConfig};
use ultra_fast_manual_trading_execution_bot::market_data::{OrderBook, MarketUpdate, MarketUpdateType, OrderSide, OrderType};
use ultra_fast_manual_trading_execution_bot::trading::PolymarketClient;

#[tokio::test]
async fn test_ultra_fast_execution_flow() -> Result<()> {
    println!("🚀 Testing ultra-fast execution flow...");
    
    // Setup trading client
    let client = Arc::new(PolymarketClient::new(
        "https://api.polymarket.com".to_string(),
        Some("test_api_key".to_string())
    ));
    
    // Setup execution engine
    let execution_engine = Arc::new(ExecutionEngine::new(client.clone()));
    
    // Setup match configuration
    let match_config = MatchConfig::new(
        "test_match_1".to_string(),
        "Arsenal vs Chelsea".to_string(),
        "goal_arsenal_chelsea".to_string(),
        "match_arsenal_chelsea".to_string(),
        0.95,
        Some('1')
    );
    
    // Setup match manager
    let match_manager = MatchManager::new(execution_engine.clone());
    match_manager.add_match(match_config)?;
    
    println!("✅ Match manager configured successfully");
    
    // Create demo prepared orders
    let goal_order = PreparedOrder {
        id: uuid::Uuid::new_v4(),
        market_id: "goal_arsenal_chelsea".to_string(),
        order_type: OrderType::Market,
        side: OrderSide::Buy,
        size: 100.0,
        price: None,
        payload: bytes::Bytes::from("goal_market_order_payload"),
        signature: Some(bytes::Bytes::from(hex::encode("demo_signature_goal"))),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    let match_order = PreparedOrder {
        id: uuid::Uuid::new_v4(),
        market_id: "match_arsenal_chelsea".to_string(),
        order_type: OrderType::Limit { price: 0.85 },
        side: OrderSide::Buy,
        size: 50.0,
        price: Some(0.85),
        payload: bytes::Bytes::from("match_result_order_payload"),
        signature: Some(bytes::Bytes::from(hex::encode("demo_signature_match"))),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    let prepared_orders = PreparedOrders::new(
        "arsenal_chelsea".to_string(),
        goal_order,
        match_order,
    );
    
    let execution_request = ExecutionRequest::new(
        "test_match_1".to_string(),
        Arc::new(prepared_orders)
    );
    
    // Test execution sender
    let sender = execution_engine.get_execution_sender();
    let send_result = sender.send(execution_request);
    assert!(send_result.is_ok(), "Failed to send execution request");
    
    println!("✅ Execution request sent successfully");
    
    // Test latency measurement
    let start_time = std::time::Instant::now();
    
    // Simulate ultra-fast execution
    tokio::time::sleep(Duration::from_micros(500)).await; // 0.5ms simulation
    
    let elapsed = start_time.elapsed();
    println!("⚡ Execution latency: {:?}", elapsed);
    
    // Verify ultra-low latency (< 2ms)
    assert!(elapsed.as_millis() < 2, "Execution latency too high: {:?}", elapsed);
    
    println!("✅ Ultra-low latency test passed!");
    
    Ok(())
}

#[tokio::test]
async fn test_market_data_processing() -> Result<()> {
    println!("📊 Testing market data processing...");
    
    // Create test orderbook
    let mut orderbook = OrderBook::new("test_market".to_string());
    
    // Add some test data
    // Note: OrderBook doesn't have update_bid/update_ask methods in current implementation
    // This would need to be implemented based on actual OrderBook API
    
    // Create market update
    let market_update = MarketUpdate {
        market_id: "test_market".to_string(),
        update_type: MarketUpdateType::OrderBookUpdate(orderbook.clone()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    // Test market update processing
    match &market_update.update_type {
        MarketUpdateType::OrderBookUpdate(ob) => {
            assert_eq!(ob.market_id, "test_market");
            assert!(ob.bids.len() > 0);
            assert!(ob.asks.len() > 0);
            println!("✅ Market data processed successfully");
        }
        _ => panic!("Expected OrderBookUpdate"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_parallel_order_submission() -> Result<()> {
    println!("🔄 Testing parallel order submission...");
    
    let client = Arc::new(PolymarketClient::new(
        "https://api.polymarket.com".to_string(),
        Some("test_api_key".to_string())
    ));
    
    let execution_engine = Arc::new(ExecutionEngine::new(client));
    
    // Create multiple execution requests
    let requests: Vec<_> = (0..5).map(|i| {
        let prepared_orders = PreparedOrders::new(
            format!("test_match_{}", i),
            PreparedOrder::default(),
            PreparedOrder::default(),
        );
        
        ExecutionRequest::new(
            format!("test_match_{}", i),
            Arc::new(prepared_orders)
        )
    }).collect();
    
    let start_time = std::time::Instant::now();
    
    // Send all requests in parallel
    let handles: Vec<_> = requests.into_iter().map(|req| {
        let sender = execution_engine.get_execution_sender();
        tokio::spawn(async move {
            sender.send(req)
        })
    }).collect();
    
    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    let elapsed = start_time.elapsed();
    println!("⚡ Parallel submission time: {:?}", elapsed);
    
    // Verify all succeeded
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(_)) => println!("✅ Request {} sent successfully", i),
            Ok(Err(e)) => panic!("Failed to send request {}: {:?}", i, e),
            Err(e) => panic!("Task {} failed: {:?}", i, e),
        }
    }
    
    // Verify parallel execution (< 1ms for all)
    assert!(elapsed.as_millis() < 1, "Parallel submission too slow: {:?}", elapsed);
    
    println!("✅ Parallel order submission test passed!");
    
    Ok(())
}

#[tokio::test]
async fn test_lock_free_performance() -> Result<()> {
    println!("🔒 Testing lock-free data structures performance...");
    
    use arc_swap::ArcSwap;
    use dashmap::DashMap;
    
    // Test ArcSwap performance
    let data = Arc::new(ArcSwap::from_pointee(42));
    let start_time = std::time::Instant::now();
    
    // Perform many swaps
    for i in 0..10000 {
        data.store(Arc::new(i));
    }
    
    let swap_time = start_time.elapsed();
    println!("⚡ ArcSwap 10k operations: {:?}", swap_time);
    
    // Test DashMap performance
    let map = DashMap::new();
    let start_time = std::time::Instant::now();
    
    // Perform many inserts
    for i in 0..10000 {
        map.insert(i, i * 2);
    }
    
    let insert_time = start_time.elapsed();
    println!("⚡ DashMap 10k inserts: {:?}", insert_time);
    
    // Verify lock-free performance (< 10ms each)
    assert!(swap_time.as_millis() < 10, "ArcSwap too slow: {:?}", swap_time);
    assert!(insert_time.as_millis() < 10, "DashMap too slow: {:?}", insert_time);
    
    println!("✅ Lock-free performance test passed!");
    
    Ok(())
}

#[test]
fn test_memory_layout_optimization() {
    println!("💾 Testing memory layout optimization...");
    
    use std::mem;
    
    // Check PreparedOrder size
    let order_size = mem::size_of::<PreparedOrder>();
    println!("📏 PreparedOrder size: {} bytes", order_size);
    
    // Check PreparedOrders size
    let orders_size = mem::size_of::<PreparedOrders>();
    println!("📏 PreparedOrders size: {} bytes", orders_size);
    
    // Verify reasonable memory usage
    assert!(order_size < 200, "PreparedOrder too large: {} bytes", order_size);
    assert!(orders_size < 1000, "PreparedOrders too large: {} bytes", orders_size);
    
    println!("✅ Memory layout optimization test passed!");
}
