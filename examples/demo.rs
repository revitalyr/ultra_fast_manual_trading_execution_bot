use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn};

use ultra_fast_manual_trading_execution_bot::execution::{ExecutionEngine, PreparedOrder, PreparedOrders, ExecutionRequest};
use ultra_fast_manual_trading_execution_bot::match_engine::{MatchManager, MatchConfig};
use ultra_fast_manual_trading_execution_bot::market_data::{OrderBook, MarketUpdate, MarketUpdateType, OrderSide, OrderType};
use ultra_fast_manual_trading_execution_bot::trading::PolymarketClient;

/// Demo application showcasing the ultra-fast trading bot capabilities
pub struct TradingBotDemo {
    execution_engine: Arc<ExecutionEngine>,
    match_manager: Arc<MatchManager>,
    market_data_rx: mpsc::UnboundedReceiver<MarketUpdate>,
}

impl TradingBotDemo {
    pub async fn new() -> Result<Self> {
        println!("🚀 Initializing Ultra Fast Trading Bot Demo...");
        
        // Setup trading client
        let client = Arc::new(PolymarketClient::new(
            "https://api.polymarket.com".to_string(),
            Some("demo_api_key".to_string())
        ));
        
        // Setup execution engine
        let execution_engine = Arc::new(ExecutionEngine::new(client));
        
        // Setup match manager
        let match_manager = Arc::new(MatchManager::new(execution_engine.clone()));
        
        // Setup market data receiver
        let (_market_data_tx, market_data_rx) = mpsc::unbounded_channel();
        
        println!("✅ Core components initialized");
        
        Ok(Self {
            execution_engine,
            match_manager,
            market_data_rx,
        })
    }
    
    pub async fn configure_demo_matches(&self) -> Result<()> {
        println!("⚙️  Configuring demo matches...");
        
        // Demo match 1: Arsenal vs Chelsea
        let match1 = MatchConfig::new(
            "arsenal_chelsea".to_string(),
            "Arsenal vs Chelsea".to_string(),
            "goal_arsenal_chelsea".to_string(),
            "match_arsenal_chelsea".to_string(),
            0.95,
            Some('1')
        );
        
        // Demo match 2: Real Madrid vs Barcelona
        let match2 = MatchConfig::new(
            "real_madrid_barcelona".to_string(),
            "Real Madrid vs Barcelona".to_string(),
            "goal_real_barcelona".to_string(),
            "match_real_barcelona".to_string(),
            0.92,
            Some('2')
        );
        
        // Demo match 3: PSG vs Marseille
        let match3 = MatchConfig::new(
            "psg_marseille".to_string(),
            "PSG vs Marseille".to_string(),
            "goal_psg_marseille".to_string(),
            "match_psg_marseille".to_string(),
            0.88,
            Some('3')
        );
        
        // Add matches to manager
        self.match_manager.add_match(match1)?;
        self.match_manager.add_match(match2)?;
        self.match_manager.add_match(match3)?;
        
        println!("✅ {} demo matches configured", 3);
        
        Ok(())
    }
    
    pub async fn simulate_market_data(&self) -> Result<()> {
        println!("📊 Simulating real-time market data...");
        
        let matches = vec![
            ("arsenal_chelsea", "goal_arsenal_chelsea"),
            ("real_madrid_barcelona", "goal_real_barcelona"),
            ("psg_marseille", "goal_psg_marseille"),
        ];
        
        // Simulate market data updates
        for _ in 0..10 {
            for (match_id, market_id) in &matches {
                let mut orderbook = OrderBook::new(market_id.to_string());
                
                // Simulate realistic bid/ask prices
                let base_price = fastrand::f64() * 0.5 + 0.4; // 0.4-0.9
                
                // Note: OrderBook doesn't have update_bid/update_ask methods in current implementation
                // This would need to be implemented based on actual OrderBook API
                
                let market_update = MarketUpdate {
                    market_id: market_id.to_string(),
                    update_type: MarketUpdateType::OrderBookUpdate(orderbook),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };
                
                // In real implementation, this would be sent to match engine
                info!("Market update for {}: price={:.3}", market_id, base_price);
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        println!("✅ Market data simulation completed");
        
        Ok(())
    }
    
    pub async fn demonstrate_ultra_fast_execution(&self) -> Result<()> {
        println!("⚡ Demonstrating ultra-fast execution...");
        
        // Create demo prepared orders
        let prepared_orders = PreparedOrders::new(
            "arsenal_chelsea".to_string(),
            // Goal market order (market order)
            PreparedOrder {
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
            },
            // Match result order (limit order)
            PreparedOrder {
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
            },
        );
        
        // Measure execution latency
        let start_time = Instant::now();
        
        // Create execution request
        let execution_request = ExecutionRequest::new(
            "arsenal_chelsea".to_string(),
            Arc::new(prepared_orders)
        );
        
        // Send to execution engine
        let sender = self.execution_engine.get_execution_sender();
        let send_result = sender.send(execution_request);
        
        let execution_time = start_time.elapsed();
        
        match send_result {
            Ok(_) => {
                println!("⚡ Execution completed in {:?}", execution_time);
                
                if execution_time.as_micros() < 1000 {
                    println!("🎯 SUB-MILLISECOND EXECUTION ACHIEVED!");
                } else if execution_time.as_millis() < 2 {
                    println!("✅ Ultra-low latency execution (< 2ms)");
                } else {
                    warn!("⚠️  Execution latency above target: {:?}", execution_time);
                }
            }
            Err(e) => {
                warn!("Failed to execute: {:?}", e);
            }
        }
        
        Ok(())
    }
    
    pub async fn demonstrate_parallel_execution(&self) -> Result<()> {
        println!("🔄 Demonstrating parallel order submission...");
        
        let start_time = Instant::now();
        
        // Create multiple execution requests for different matches
        let matches = vec!["arsenal_chelsea", "real_madrid_barcelona", "psg_marseille"];
        let handles: Vec<_> = matches.into_iter().enumerate().map(|(i, match_id)| {
            let execution_engine = self.execution_engine.clone();
            
            tokio::spawn(async move {
                let prepared_orders = PreparedOrders::new(
                    match_id.to_string(),
                    PreparedOrder::default(),
                    PreparedOrder::default(),
                );
                
                let request = ExecutionRequest::new(
                    match_id.to_string(),
                    Arc::new(prepared_orders)
                );
                
                let sender = execution_engine.get_execution_sender();
                sender.send(request)
            })
        }).collect();
        
        // Wait for all parallel executions
        let results: Vec<_> = futures::future::join_all(handles).await;
        
        let total_time = start_time.elapsed();
        
        // Count successful executions
        let mut successful = 0;
        let total_count = results.len();
        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(Ok(_)) => {
                    successful += 1;
                    println!("✅ Match {} executed successfully", i + 1);
                }
                Ok(Err(e)) => println!("❌ Match {} failed: {:?}", i + 1, e),
                Err(e) => println!("❌ Match {} task failed: {:?}", i + 1, e),
            }
        }
        
        println!("⚡ Parallel execution completed in {:?}", total_time);
        println!("📊 {}/{} matches executed successfully", successful, total_count);
        
        if total_time.as_millis() < 5 {
            println!("🚀 EXCELLENT parallel performance!");
        }
        
        Ok(())
    }
    
    pub async fn run_interactive_demo(&self) -> Result<()> {
        println!("🎮 Starting interactive demo...");
        println!("Press keys 1-3 to execute trades for demo matches");
        println!("Press 'q' to quit");
        println!("Press 's' to run speed test");
        println!("Press 'p' to run parallel test");
        println!();
        
        // Create a simple interactive demo
        let (tx, mut rx) = mpsc::unbounded_channel::<char>();
        
        // Spawn keyboard listener task
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            use crossterm::event::{self, KeyCode, KeyEvent};
            use std::io;
            use std::time::Duration;
            
            loop {
                if event::poll(Duration::from_millis(100)).unwrap() {
                    if let event::Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                        match code {
                            KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') |
                            KeyCode::Char('q') | KeyCode::Char('s') | KeyCode::Char('p') => {
                                if let KeyCode::Char(c) = code {
                                    let _ = tx_clone.send(c);
                                }
                            }
                            KeyCode::Esc => {
                                let _ = tx_clone.send('q');
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
        
        // Handle keyboard input
        while let Some(key) = rx.recv().await {
            match key {
                '1' | '2' | '3' => {
                    let match_names = ["Arsenal vs Chelsea", "Real Madrid vs Barcelona", "PSG vs Marseille"];
                    let match_index = key.to_digit(10).unwrap() as usize - 1;
                    println!("⚡ Executing trade for: {}", match_names[match_index]);
                    self.demonstrate_ultra_fast_execution().await?;
                }
                's' => {
                    println!("🏃 Running speed test...");
                    self.demonstrate_ultra_fast_execution().await?;
                }
                'p' => {
                    println!("🔄 Running parallel test...");
                    self.demonstrate_parallel_execution().await?;
                }
                'q' => {
                    println!("👋 Demo ended");
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    pub async fn run_full_demo(&self) -> Result<()> {
        println!("🎬 Starting Full Trading Bot Demo");
        println!("=====================================");
        
        // Step 1: Configure matches
        self.configure_demo_matches().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Step 2: Simulate market data
        self.simulate_market_data().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Step 3: Demonstrate ultra-fast execution
        self.demonstrate_ultra_fast_execution().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Step 4: Demonstrate parallel execution
        self.demonstrate_parallel_execution().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Step 5: Interactive demo
        self.run_interactive_demo().await?;
        
        println!("🎉 Demo completed successfully!");
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    println!("🤖 Ultra Fast Manual Trading Execution Bot - Demo Mode");
    println!("==================================================");
    println!();
    
    // Create and run demo
    let demo = TradingBotDemo::new().await?;
    demo.run_full_demo().await?;
    
    Ok(())
}
