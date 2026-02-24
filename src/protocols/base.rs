//! Base protocol trait for exchange connections

use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::types::{
    ExchangeId, ExchangeConfig, Symbol, ExchangeData, 
    ConnectionHealth, OrderRequest, OrderId
};

/// Base trait for all exchange connections
/// 
/// This trait provides a unified interface for connecting to different exchanges
/// regardless of their underlying protocol (TCP, WebSocket, FIX, etc.).
#[async_trait]
pub trait ExchangeConnection: Send + Sync {
    /// Connect to the exchange
    async fn connect(&mut self) -> Result<()>;
    
    /// Disconnect from the exchange
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Check if the connection is currently active
    fn is_connected(&self) -> bool;
    
    /// Perform a health check on the connection
    async fn health_check(&self) -> Result<ConnectionHealth>;
    
    /// Subscribe to market data for the given symbols
    async fn subscribe_market_data(&mut self, symbols: &[Symbol]) -> Result<()>;
    
    /// Unsubscribe from market data for the given symbols
    async fn unsubscribe_market_data(&mut self, symbols: &[Symbol]) -> Result<()>;
    
    /// Submit an order for execution
    async fn submit_order(&mut self, order: OrderRequest) -> Result<OrderId>;
    
    /// Cancel an existing order
    async fn cancel_order(&mut self, order_id: OrderId) -> Result<()>;
    
    /// Get the exchange ID for this connection
    fn exchange_id(&self) -> ExchangeId;
    
    /// Get connection statistics
    fn get_stats(&self) -> ConnectionStats;
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub messages_received: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub bytes_sent: u64,
    pub last_message_time: Option<std::time::SystemTime>,
    pub connection_time: Option<std::time::SystemTime>,
    pub error_count: u64,
    pub reconnect_count: u64,
}

/// Factory for creating exchange connections
pub trait ConnectionFactory: Send + Sync {
    /// Create a new connection with the given configuration
    fn create_connection(&self, config: ExchangeConfig) -> Result<Arc<dyn ExchangeConnection>>;
    
    /// Get the exchange ID this factory supports
    fn exchange_id(&self) -> ExchangeId;
    
    /// Get supported features
    fn supported_features(&self) -> Vec<ExchangeFeature>;
}

/// Exchange capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeFeature {
    MarketData,
    OrderExecution,
    HistoricalData,
    AccountData,
    RealTimeNews,
    OptionsTrading,
    FuturesTrading,
    CryptoTrading,
}

/// Message channel for exchange data
pub type DataReceiver = mpsc::UnboundedReceiver<ExchangeData>;
pub type DataSender = mpsc::UnboundedSender<ExchangeData>;

/// Base implementation for exchange connections
pub struct BaseExchangeConnection {
    exchange_id: ExchangeId,
    config: ExchangeConfig,
    stats: ConnectionStats,
    data_sender: Option<DataSender>,
    is_connected: bool,
}

impl BaseExchangeConnection {
    /// Create a new base connection
    pub fn new(exchange_id: ExchangeId, config: ExchangeConfig) -> Self {
        Self {
            exchange_id,
            config,
            stats: ConnectionStats::default(),
            data_sender: None,
            is_connected: false,
        }
    }
    
    /// Get the exchange ID
    pub fn exchange_id(&self) -> ExchangeId {
        self.exchange_id.clone()
    }
    
    /// Get the configuration
    pub fn config(&self) -> &ExchangeConfig {
        &self.config
    }
    
    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        self.stats.clone()
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
    
    /// Set connection status
    pub fn set_connected(&mut self, connected: bool) {
        self.is_connected = connected;
        if connected {
            self.stats.connection_time = Some(std::time::SystemTime::now());
        }
    }
    
    /// Update statistics for received message
    pub fn update_received_stats(&mut self, bytes: usize) {
        self.stats.messages_received += 1;
        self.stats.bytes_received += bytes as u64;
        self.stats.last_message_time = Some(std::time::SystemTime::now());
    }
    
    /// Update statistics for sent message
    pub fn update_sent_stats(&mut self, bytes: usize) {
        self.stats.messages_sent += 1;
        self.stats.bytes_sent += bytes as u64;
    }
    
    /// Increment error count
    pub fn increment_error_count(&mut self) {
        self.stats.error_count += 1;
    }
    
    /// Increment reconnect count
    pub fn increment_reconnect_count(&mut self) {
        self.stats.reconnect_count += 1;
    }
    
    /// Set up data channel
    pub fn setup_data_channel(&mut self) -> DataReceiver {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.data_sender = Some(sender);
        receiver
    }
    
    /// Send data through the channel
    pub fn send_data(&self, data: ExchangeData) -> Result<()> {
        if let Some(ref sender) = self.data_sender {
            sender.send(data).map_err(|e| anyhow::anyhow!("Failed to send data: {}", e))?;
        }
        Ok(())
    }
}

/// Reconnection logic for exchange connections
pub struct ReconnectHandler {
    max_attempts: u32,
    delay_ms: u64,
    current_attempts: u32,
}

impl ReconnectHandler {
    pub fn new(max_attempts: u32, delay_ms: u64) -> Self {
        Self {
            max_attempts,
            delay_ms,
            current_attempts: 0,
        }
    }
    
    /// Attempt to reconnect with exponential backoff
    pub async fn attempt_reconnect<F, Fut>(&mut self, reconnect_fn: F) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        if self.current_attempts >= self.max_attempts {
            return Err(anyhow::anyhow!("Max reconnection attempts reached"));
        }
        
        self.current_attempts += 1;
        
        // Exponential backoff
        let delay = self.delay_ms * 2_u64.pow(self.current_attempts - 1);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        
        reconnect_fn().await
    }
    
    /// Reset the reconnection attempt counter
    pub fn reset(&mut self) {
        self.current_attempts = 0;
    }
    
    /// Check if more attempts are available
    pub fn has_attempts_left(&self) -> bool {
        self.current_attempts < self.max_attempts
    }
}
