use crate::market_data::orderbook::{MarketUpdate, OrderBook};
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

pub struct MarketDataListener {
    websocket_url: String,
    update_tx: mpsc::UnboundedSender<MarketUpdate>,
}

impl MarketDataListener {
    pub fn new(websocket_url: String, update_tx: mpsc::UnboundedSender<MarketUpdate>) -> Self {
        Self {
            websocket_url,
            update_tx,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.websocket_url).await?;
        info!("Connected to market data WebSocket: {}", self.websocket_url);

        let (mut write, mut read): (_, _) = ws_stream.split();
        let tx = self.update_tx.clone();

        // Subscribe to markets
        let subscribe_msg = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channels": ["orderbook", "trades", "liquidity"]
            }
        });

        write.send(Message::Text(subscribe_msg.to_string())).await?;

        // Handle incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Some(update) = self.parse_message(&text) {
                        if let Err(e) = tx.send(update) {
                            error!("Failed to send market update: {}", e);
                            break;
                        }
                    }
                }
                Ok(Message::Binary(data)) => {
                    warn!("Received binary data, ignoring");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_message(&self, message: &str) -> Option<MarketUpdate> {
        let value: Value = serde_json::from_str(message).ok()?;
        
        let channel = value.get("channel")?.as_str()?;
        let data = value.get("data")?;
        let timestamp = value.get("timestamp")?.as_u64().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });

        match channel {
            "orderbook" => {
                let orderbook: OrderBook = serde_json::from_value(data.clone()).ok()?;
                Some(MarketUpdate {
                    market_id: orderbook.market_id.clone(),
                    update_type: crate::market_data::orderbook::MarketUpdateType::OrderBookUpdate(orderbook),
                    timestamp,
                })
            }
            "trades" => {
                // Parse trade data
                // This would be implemented based on actual Polymarket API format
                None // Placeholder
            }
            "liquidity" => {
                // Parse liquidity data
                // This would be implemented based on actual Polymarket API format
                None // Placeholder
            }
            _ => None,
        }
    }
}

pub struct MarketDataManager {
    listeners: Vec<Arc<MarketDataListener>>,
}

impl MarketDataManager {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<MarketUpdate>) {
        let (_update_tx, update_rx) = mpsc::unbounded_channel();
        let manager = Self {
            listeners: Vec::new(),
        };
        (manager, update_rx)
    }

    pub fn add_listener(&mut self, websocket_url: String) -> mpsc::UnboundedSender<MarketUpdate> {
        let (update_tx, _) = mpsc::unbounded_channel();
        let listener = Arc::new(MarketDataListener::new(websocket_url, update_tx.clone()));
        self.listeners.push(listener);
        update_tx
    }

    pub async fn start_all(&self) -> Result<()> {
        let handles: Vec<_> = self.listeners
            .iter()
            .map(|listener| {
                let listener = listener.clone();
                tokio::spawn(async move {
                    if let Err(e) = listener.start().await {
                        error!("Market data listener error: {}", e);
                    }
                })
            })
            .collect();

        // Wait for all listeners
        futures::future::join_all(handles).await;
        Ok(())
    }
}
