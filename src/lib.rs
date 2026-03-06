//! Ultra Fast Manual Trading Execution Bot
//! 
//! A high-performance Rust trading bot designed for ultra-low latency 
//! manual execution on Polymarket. The system allows users to manually 
//! trigger trades by pressing keyboard shortcuts, with orders pre-built 
//! and pre-signed for sub-millisecond execution times.
//!
//! # Features
//!
//! - **Ultra-Low Latency Execution**: Pre-built orders stored in lock-free memory structures
//! - **Parallel Order Submission**: Both goal market and match result orders sent simultaneously
//! - **Multi-Match Support**: Handle multiple matches simultaneously with individual keyboard shortcuts
//! - **Keyboard Interface**: Simple terminal-based UI for instant execution
//! - **HFT-Style Architecture**: Zero-allocation execution path with lock-free data structures
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use ultra_fast_manual_trading_execution_bot::*;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize components
//!     let client = Arc::new(PolymarketClient::new(
//!         "https://api.polymarket.com".to_string(),
//!         Some("api_key".to_string())
//!     ));
//!     let execution_engine = Arc::new(ExecutionEngine::new(client));
//!     let match_manager = MatchManager::new(execution_engine);
//!     
//!     // Configure matches
//!     let match_config = MatchConfig::new(
//!         "match_1".to_string(),
//!         "Team A vs Team B".to_string(),
//!         "goal_market".to_string(),
//!         "match_result_market".to_string(),
//!         0.95,
//!         Some('1')
//!     );
//!     match_manager.add_match(match_config)?;
//!     
//!     // Start trading dashboard
//!     let mut dashboard = KeyboardDashboard::new(match_manager);
//!     dashboard.run().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod execution;
pub mod market_data;
pub mod match_engine;
pub mod trading;
pub mod ui;

// Re-export commonly used types
pub use execution::{ExecutionEngine, PreparedOrder, PreparedOrders, ExecutionRequest, ExecutionResult};
pub use market_data::{OrderBook, MarketUpdate, OrderSide, OrderType};
pub use match_engine::{MatchEngine, MatchManager, MatchConfig};
pub use trading::PolymarketClient;
pub use ui::KeyboardDashboard;
