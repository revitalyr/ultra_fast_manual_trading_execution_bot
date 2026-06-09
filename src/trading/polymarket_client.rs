use crate::execution::prepared_orders::PreparedOrder;
use crate::traits::TradingClient;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, info};

const POOL_IDLE_TIMEOUT_SECS: u64 = 60;
const POOL_MAX_IDLE_PER_HOST: usize = 10;
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub struct PolymarketClient {
    client: Client,
    base_url: String,
    api_key: Option<SecretString>,
}

impl PolymarketClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
            .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
            .timeout(Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            api_key: api_key.map(SecretString::from),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/{}", self.base_url, path)
    }

    async fn check_response(
        &self,
        response: reqwest::Response,
        context: &str,
    ) -> Result<reqwest::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            error!("{} — HTTP {}", context, status);
            let error_msg = format!("{} (HTTP {}): {}", context, status, error_text);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    pub async fn submit_order(&self, order: &PreparedOrder) -> Result<Value> {
        let url = self.url("orders");

        // Build payload with fresh timestamp at execution time
        let payload = order.build_payload()?;

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(payload);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        debug!("Submitting order to market: {}", order.market_id);

        let response = self
            .check_response(request.send().await?, "Order submission failed")
            .await?;
        let result: Value = response.json().await?;
        info!("Order submitted successfully: {}", order.id);
        Ok(result)
    }

    async fn submit_prepared_order_internal(
        &self,
        payload: &Bytes,
        signature: Option<&Bytes>,
    ) -> Result<Value> {
        let url = self.url("orders/submit");

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(payload.clone());

        if let Some(sig) = signature {
            request = request.header("X-Signature", hex::encode(sig));
        }

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        let response = self
            .check_response(request.send().await?, "Prepared order submission failed")
            .await?;
        let result: Value = response.json().await?;
        debug!("Prepared order submitted successfully");
        Ok(result)
    }

    async fn get_markets_internal(&self) -> Result<Vec<Value>> {
        let url = self.url("markets");

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        let response = self
            .check_response(request.send().await?, "Failed to get markets")
            .await?;
        let markets: Vec<Value> = response.json().await?;
        info!("Retrieved {} markets", markets.len());
        Ok(markets)
    }

    async fn get_orderbook_internal(&self, market_id: &str) -> Result<Value> {
        let url = self.url(&format!("markets/{}/orderbook", market_id));

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        let response = self
            .check_response(request.send().await?, "Failed to get orderbook")
            .await?;
        let orderbook: Value = response.json().await?;
        debug!("Retrieved orderbook for market: {}", market_id);
        Ok(orderbook)
    }

    async fn get_balance_internal(&self) -> Result<Value> {
        let url = self.url("account/balance");

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        let response = self
            .check_response(request.send().await?, "Failed to get balance")
            .await?;
        let balance: Value = response.json().await?;
        debug!("Retrieved account balance");
        Ok(balance)
    }

    async fn cancel_order_internal(&self, order_id: &str) -> Result<Value> {
        let url = self.url(&format!("orders/{}/cancel", order_id));

        let mut request = self.client.post(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        let response = self
            .check_response(request.send().await?, "Failed to cancel order")
            .await?;
        let result: Value = response.json().await?;
        info!("Order cancelled successfully: {}", order_id);
        Ok(result)
    }
}

#[async_trait]
impl TradingClient for PolymarketClient {
    async fn submit_order(&self, order: &PreparedOrder) -> Result<Value> {
        // Call the public method directly
        let url = self.url("orders");

        // Build payload with fresh timestamp at execution time
        let payload = order.build_payload()?;

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(payload);

        if let Some(api_key) = &self.api_key {
            request = request.header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            );
        }

        debug!("Submitting order to market: {}", order.market_id);

        let response = self
            .check_response(request.send().await?, "Order submission failed")
            .await?;
        let result: Value = response.json().await?;
        info!("Order submitted successfully: {}", order.id);
        Ok(result)
    }

    async fn submit_prepared_order(
        &self,
        payload: &Bytes,
        signature: Option<&Bytes>,
    ) -> Result<Value> {
        self.submit_prepared_order_internal(payload, signature)
            .await
    }

    async fn get_markets(&self) -> Result<Vec<Value>> {
        self.get_markets_internal().await
    }

    async fn get_orderbook(&self, market_id: &str) -> Result<Value> {
        self.get_orderbook_internal(market_id).await
    }

    async fn get_balance(&self) -> Result<Value> {
        self.get_balance_internal().await
    }

    async fn cancel_order(&self, order_id: &str) -> Result<Value> {
        self.cancel_order_internal(order_id).await
    }
}

// Public wrapper methods for backward compatibility
impl PolymarketClient {
    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn submit_prepared_order(
        &self,
        payload: &Bytes,
        signature: Option<&Bytes>,
    ) -> Result<Value> {
        TradingClient::submit_prepared_order(self, payload, signature).await
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_markets(&self) -> Result<Vec<Value>> {
        TradingClient::get_markets(self).await
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_orderbook(&self, market_id: &str) -> Result<Value> {
        TradingClient::get_orderbook(self, market_id).await
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn get_balance(&self) -> Result<Value> {
        TradingClient::get_balance(self).await
    }

    #[allow(dead_code)] // This method is part of the full API but not used in the current demo flow
    pub async fn cancel_order(&self, order_id: &str) -> Result<Value> {
        TradingClient::cancel_order(self, order_id).await
    }
}
