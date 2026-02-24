//! Exchange Connection Hub
//!
//! A unified exchange connection system for HFT trading applications.
//! Provides protocol-agnostic interface for connecting to various exchanges
//! and forwarding market data through the market-data-engine and hft-event-bus.
//!
//! # Features
//! - Zero-allocation market data processing
//! - Multi-exchange support with unified interface
//! - Protocol agnostic design
//! - Health monitoring and automatic failover
//! - Integration with market-data-engine and hft-event-bus
//!
//! # Quick Start
//! ```rust
//! use exchange_connection_hub::ExchangeHub;
//! use hft_event_bus::typed_bus::TypedEventBus;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let event_bus = std::sync::Arc::new(TypedEventBus::new());
//!     let mut hub = ExchangeHub::new(event_bus);
//!     
//!     // Connect to NinjaTrader
//!     let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
//!     hub.connect_exchange(ExchangeId::NinjaTrader, config).await?;
//!     
//!     // Subscribe to market data
//!     hub.subscribe_market_data(ExchangeId::NinjaTrader, &["ES", "NQ"]).await?;
//!     
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, debug};

pub mod protocols;
pub mod types;
pub mod manager;
pub mod registry;
pub mod adapters;

use adapters::market_data::MarketDataBridge;
use manager::ExchangeConnectionManager;
use registry::ExchangeConnectionRegistry;
use types::{ExchangeId, ExchangeConfig, Symbol, ExchangeData};
use hft_event_bus::typed_bus::TypedEventBus;
use market_data_engine::handlers::EventProcessor;

/// Main exchange connection hub that orchestrates all exchange connections
/// and forwards market data through the processing pipeline.
pub struct ExchangeHub {
    connection_manager: ExchangeConnectionManager,
    market_data_bridge: MarketDataBridge,
    event_processor: Arc<market_data_engine::handlers::EventProcessor>,
    event_bus: Arc<hft_event_bus::typed_bus::TypedEventBus>,
    registry: ExchangeConnectionRegistry,
}

impl ExchangeHub {
    /// Create a new exchange hub with the given event bus
    pub fn new(event_bus: Arc<hft_event_bus::typed_bus::TypedEventBus>) -> Self {
        let event_processor = Arc::new(market_data_engine::handlers::EventProcessor::warmup());
        let market_data_bridge = MarketDataBridge::new(event_processor.clone(), event_bus.clone());
        
        Self {
            connection_manager: ExchangeConnectionManager::new(),
            market_data_bridge,
            event_processor,
            event_bus,
            registry: ExchangeConnectionRegistry::default(),
        }
    }
    
    /// Connect to an exchange with the given configuration
    pub async fn connect_exchange(&mut self, exchange_id: ExchangeId, config: ExchangeConfig) -> Result<()> {
        info!("Connecting to exchange: {:?}", exchange_id);
        
        let connection = self.registry.create_connection(exchange_id.clone(), config)?;
        let exchange_id_for_log = exchange_id.clone();
        self.connection_manager.add_connection(exchange_id, connection).await?;
        
        info!("Connected to exchange: {:?}", exchange_id_for_log);
        Ok(())
    }
    
    /// Subscribe to market data for the given symbols on the specified exchange
    pub async fn subscribe_market_data(&mut self, exchange_id: ExchangeId, symbols: &[&str]) -> Result<()> {
        let symbols: Vec<Symbol> = symbols.iter().map(|s| (*s).to_string()).collect();
        
        info!("Subscribing to {} symbols on exchange: {:?}", symbols.len(), exchange_id);
        self.connection_manager.subscribe(exchange_id.clone(), &symbols).await?;
        
        info!("Successfully subscribed to symbols on exchange: {:?}", exchange_id);
        Ok(())
    }
    
    /// Get connection status for all exchanges
    pub fn connection_status(&self) -> Vec<(ExchangeId, bool)> {
        self.connection_manager.connection_status()
    }
    
    /// Get connection status for a specific exchange
    pub fn get_connection_status(&self, exchange_id: ExchangeId) -> Option<types::ConnectionStatus> {
        self.connection_manager.get_connection_status(exchange_id)
    }
    
    /// Process exchange data (internal method used by connection manager)
    pub(crate) async fn process_exchange_data(&self, exchange_id: ExchangeId, data: ExchangeData) -> Result<()> {
        match data {
            ExchangeData::Trade(trade) => {
                self.market_data_bridge.process_trade(trade)?;
            }
            ExchangeData::Quote(quote) => {
                self.market_data_bridge.process_quote(quote)?;
            }
            ExchangeData::OrderBook(_order_book) => {
                debug!("Received order book data (not yet published to event bus)");
            }
            ExchangeData::Metadata(_metadata) => {
                debug!("Received metadata (not yet published to event bus)");
            }
        }
        
        Ok(())
    }
}

impl Clone for ExchangeHub {
    fn clone(&self) -> Self {
        Self {
            connection_manager: ExchangeConnectionManager::new(),
            market_data_bridge: MarketDataBridge::new(self.event_processor.clone(), self.event_bus.clone()),
            event_processor: self.event_processor.clone(),
            event_bus: self.event_bus.clone(),
            registry: ExchangeConnectionRegistry::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_exchange_hub_creation() {
        let hub = ExchangeHub::new();
        
        // Verify hub is created successfully
        assert_eq!(hub.connection_status().len(), 0);
    }
}
