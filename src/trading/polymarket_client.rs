use crate::execution::prepared_orders::PreparedOrder;
use anyhow::Result;
use bytes::Bytes;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct PolymarketClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl PolymarketClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            api_key,
        }
    }

    pub async fn submit_order(&self, order: &PreparedOrder) -> Result<Value> {
        let url = format!("{}/api/v1/orders", self.base_url);
        
        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(order.payload.clone());

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        debug!("Submitting order to market: {}", order.market_id);

        let response = request.send().await?;
        
        if response.status().is_success() {
            let result: Value = response.json().await?;
            info!("Order submitted successfully: {}", order.id);
            debug!("Order response: {}", result);
            Ok(result)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Order submission failed: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn submit_prepared_order(
        &self,
        payload: &Bytes,
        signature: Option<&Bytes>,
    ) -> Result<Value> {
        let url = format!("{}/api/v1/orders/submit", self.base_url);
        
        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(payload.clone());

        if let Some(sig) = signature {
            request = request.header("X-Signature", hex::encode(sig));
        }

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;
        
        if response.status().is_success() {
            let result: Value = response.json().await?;
            debug!("Prepared order submitted successfully");
            Ok(result)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Prepared order submission failed: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_markets(&self) -> Result<Vec<Value>> {
        let url = format!("{}/api/v1/markets", self.base_url);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;
        
        if response.status().is_success() {
            let markets: Vec<Value> = response.json().await?;
            info!("Retrieved {} markets", markets.len());
            Ok(markets)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Failed to get markets: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_orderbook(&self, market_id: &str) -> Result<Value> {
        let url = format!("{}/api/v1/markets/{}/orderbook", self.base_url, market_id);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;
        
        if response.status().is_success() {
            let orderbook: Value = response.json().await?;
            debug!("Retrieved orderbook for market: {}", market_id);
            Ok(orderbook)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Failed to get orderbook: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_balance(&self) -> Result<Value> {
        let url = format!("{}/api/v1/account/balance", self.base_url);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;
        
        if response.status().is_success() {
            let balance: Value = response.json().await?;
            debug!("Retrieved account balance");
            Ok(balance)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Failed to get balance: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn cancel_order(&self, order_id: &str) -> Result<Value> {
        let url = format!("{}/api/v1/orders/{}/cancel", self.base_url, order_id);
        
        let mut request = self.client.post(&url);
        
        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;
        
        if response.status().is_success() {
            let result: Value = response.json().await?;
            info!("Order cancelled successfully: {}", order_id);
            Ok(result)
        } else {
            let error_text = response.text().await?;
            let error_msg = format!("Failed to cancel order: {}", error_text);
            error!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }
}
