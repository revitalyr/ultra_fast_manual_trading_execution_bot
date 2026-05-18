mod market_data;
mod execution;
mod trading;
mod match_engine;
mod ui;

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber;

use execution::ExecutionEngine;
use trading::PolymarketClient;
use match_engine::{MatchConfig, MatchManager};
use ui::KeyboardDashboard;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Ultra Fast Manual Trading Execution Bot");

    // Initialize trading client
    let polymarket_url = std::env::var("POLYMARKET_API_URL")
        .unwrap_or_else(|_| "https://api.polymarket.com".to_string());
    let api_key = std::env::var("POLYMARKET_API_KEY").ok();

    let trading_client = Arc::new(PolymarketClient::new(polymarket_url, api_key));
    info!("Trading client initialized");

    // Initialize execution engine
    let execution_engine = Arc::new(ExecutionEngine::new(trading_client.clone()));
    info!("Execution engine initialized");

    // Initialize match manager
    let match_manager = Arc::new(MatchManager::new(execution_engine.clone()));

    // Configure matches (these would come from config file in production)
    configure_matches(&match_manager).await?;

    // Start all match engines
    match_manager.start_all().await?;
    info!("All match engines started");

    // Start keyboard dashboard
    let mut dashboard = KeyboardDashboard::new(match_manager.clone());
    
    info!("Application ready. Use keyboard shortcuts to execute trades.");
    info!("Press 1-9 to execute matches, Q to quit.");

    // Run the dashboard (this is the main UI loop)
    if let Err(e) = dashboard.run().await {
        error!("Dashboard error: {}", e);
    }

    info!("Application shutdown complete");
    Ok(())
}

async fn configure_matches(match_manager: &Arc<MatchManager>) -> Result<()> {
    // Example matches - in production these would be loaded from config
    let matches = vec![
        MatchConfig::new(
            "match_1".to_string(),
            "Arsenal vs Chelsea".to_string(),
            "goal_market_arsenal_chelsea".to_string(),
            "match_result_arsenal_chelsea".to_string(),
            0.95, // Max price limit
            Some('1'), // Keyboard shortcut
        ),
        MatchConfig::new(
            "match_2".to_string(),
            "Real Madrid vs Barcelona".to_string(),
            "goal_market_real_barcelona".to_string(),
            "match_result_real_barcelona".to_string(),
            0.95,
            Some('2'),
        ),
        MatchConfig::new(
            "match_3".to_string(),
            "PSG vs Marseille".to_string(),
            "goal_market_psg_marseille".to_string(),
            "match_result_psg_marseille".to_string(),
            0.95,
            Some('3'),
        ),
    ];

    for match_config in matches {
        match_manager.add_match(match_config).await?;
        info!("Match configured successfully");
    }

    info!("All matches configured successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_engine_creation() {
        let client = Arc::new(PolymarketClient::new("test_url".to_string(), None));
        let engine = ExecutionEngine::new(client);
        // UnboundedSender doesn't have capacity() method, so just check it's not None
        let result = engine.get_execution_sender().send(ExecutionRequest::new(
            "test".to_string(), 
            Arc::new(crate::execution::prepared_orders::PreparedOrders::new(
                "test".to_string(),
                Default::default(),
                Default::default(),
            ))
        )).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_config_creation() {
        let config = MatchConfig::new(
            "test".to_string(),
            "Test Match".to_string(),
            "goal_test".to_string(),
            "match_test".to_string(),
            0.95,
            Some('t'),
        );
        assert_eq!(config.name, "Test Match");
        assert_eq!(config.keyboard_shortcut, Some('t'));
    }
}
