mod market_data;
mod execution;
mod trading;
mod match_engine;
mod ui;
mod config;
mod traits;
mod util;

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber;

use execution::ExecutionEngine;
use trading::PolymarketClient;
use match_engine::MatchManager;
use ui::KeyboardDashboard;
use config::Config;

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

    // Load match configuration from file
    let config = Config::from_default_path()
        .unwrap_or_else(|e| {
            info!("Failed to load config file: {}. Using fallback configuration.", e);
            Config::from_file("config.example.toml").unwrap_or_else(|_| {
                info!("No config file found. Creating default configuration.");
                create_default_config()
            })
        });

    // Configure matches from config
    configure_matches_from_config(&match_manager, &config);

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

fn configure_matches_from_config(match_manager: &Arc<MatchManager>, config: &Config) {
    let match_configs = config.get_match_configs();
    
    for match_config in match_configs {
        let match_name = match_config.name.clone();
        match_manager.add_match(match_config);
        info!("Match configured: {}", match_name);
    }

    info!("All matches configured successfully");
}

fn create_default_config() -> Config {
    Config {
        matches: vec![
            config::MatchConfig {
                id: "match_1".to_string(),
                name: "Arsenal vs Chelsea".to_string(),
                goal_market_id: "goal_market_arsenal_chelsea".to_string(),
                match_market_id: "match_result_arsenal_chelsea".to_string(),
                max_price_limit: 0.95,
                keyboard_shortcut: Some('1'),
            },
            config::MatchConfig {
                id: "match_2".to_string(),
                name: "Real Madrid vs Barcelona".to_string(),
                goal_market_id: "goal_market_real_barcelona".to_string(),
                match_market_id: "match_result_real_barcelona".to_string(),
                max_price_limit: 0.95,
                keyboard_shortcut: Some('2'),
            },
            config::MatchConfig {
                id: "match_3".to_string(),
                name: "PSG vs Marseille".to_string(),
                goal_market_id: "goal_market_psg_marseille".to_string(),
                match_market_id: "match_result_psg_marseille".to_string(),
                max_price_limit: 0.95,
                keyboard_shortcut: Some('3'),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_engine_creation() {
        let client = Arc::new(PolymarketClient::new("test_url".to_string(), None));
        let engine = ExecutionEngine::new(client);
        // Bounded sender has async send with backpressure
        let result = engine.get_execution_sender().send(crate::execution::ExecutionRequest::new(
            "test".to_string(), 
            Arc::new(crate::execution::PreparedOrders::new(
                "test".to_string(),
                crate::execution::PreparedOrder::placeholder(),
                crate::execution::PreparedOrder::placeholder(),
            ))
        )).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_config_creation() {
        let config = crate::match_engine::MatchConfig::new(
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
