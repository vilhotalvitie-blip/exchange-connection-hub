//! Type definitions for the exchange connection hub

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an exchange
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExchangeId {
    NinjaTrader,
    InteractiveBrokers,
    Databento,
    WebSocket(String), // For generic WebSocket exchanges
    FIX(String),       // For FIX protocol exchanges
}

impl std::fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeId::NinjaTrader => write!(f, "ninjatrader"),
            ExchangeId::InteractiveBrokers => write!(f, "ibkr"),
            ExchangeId::Databento => write!(f, "databento"),
            ExchangeId::WebSocket(name) => write!(f, "websocket:{}", name),
            ExchangeId::FIX(name) => write!(f, "fix:{}", name),
        }
    }
}

/// Configuration for connecting to an exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub exchange_id: ExchangeId,
    pub connection_string: String,
    pub credentials: Option<Credentials>,
    pub settings: HashMap<String, serde_json::Value>,
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
}

impl ExchangeConfig {
    /// Create a NinjaTrader configuration
    pub fn ninjatrader(address: &str) -> Self {
        Self {
            exchange_id: ExchangeId::NinjaTrader,
            connection_string: address.to_string(),
            credentials: None,
            settings: HashMap::new(),
            reconnect_attempts: 3,
            reconnect_delay_ms: 5000,
        }
    }
    
    /// Create an Interactive Brokers configuration
    pub fn interactive_brokers(address: &str, client_id: i32) -> Self {
        let mut settings = HashMap::new();
        settings.insert("client_id".to_string(), serde_json::Value::Number(client_id.into()));
        
        Self {
            exchange_id: ExchangeId::InteractiveBrokers,
            connection_string: address.to_string(),
            credentials: None,
            settings,
            reconnect_attempts: 3,
            reconnect_delay_ms: 5000,
        }
    }
    
    /// Create a WebSocket configuration
    pub fn websocket(name: &str, url: &str) -> Self {
        Self {
            exchange_id: ExchangeId::WebSocket(name.to_string()),
            connection_string: url.to_string(),
            credentials: None,
            settings: HashMap::new(),
            reconnect_attempts: 3,
            reconnect_delay_ms: 5000,
        }
    }
    
    /// Create a FIX configuration
    pub fn fix(name: &str, connection_string: &str) -> Self {
        Self {
            exchange_id: ExchangeId::FIX(name.to_string()),
            connection_string: connection_string.to_string(),
            credentials: None,
            settings: HashMap::new(),
            reconnect_attempts: 3,
            reconnect_delay_ms: 5000,
        }
    }
}

/// Exchange credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

/// Symbol identifier
pub type Symbol = String;

/// Exchange-specific data types
#[derive(Debug, Clone)]
pub enum ExchangeData {
    Trade(market_data_engine::types::TradeV2),
    Quote(market_data_engine::types::QuoteV2),
    OrderBook(market_data_engine::types::OrderBookSnapshot),
    Metadata(market_data_engine::types::InstrumentMetadata),
}

/// Connection health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Unhealthy,
    Disconnected,
    Reconnecting,
}

impl ConnectionHealth {
    pub fn is_healthy(self) -> bool {
        matches!(self, ConnectionHealth::Healthy)
    }
}

/// Connection status information
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub exchange_id: ExchangeId,
    pub health: ConnectionHealth,
    pub last_message_time: Option<std::time::SystemTime>,
    pub messages_received: u64,
    pub bytes_received: u64,
    pub error_count: u64,
}

/// Order request for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: Symbol,
    pub side: OrderSide,
    pub quantity: u64,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub time_in_force: TimeInForce,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GTC, // Good Till Canceled
    IOC, // Immediate Or Cancel
    FOK, // Fill Or Kill
}

/// Order identifier
pub type OrderId = String;

/// Order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatus {
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub side: OrderSide,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub price: Option<f64>,
    pub status: OrderStatusType,
    pub timestamp: i64,
}

/// Order status type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatusType {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}
