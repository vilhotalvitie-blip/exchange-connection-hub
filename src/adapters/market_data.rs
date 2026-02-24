//! Market data bridge for integration with market-data-engine

use std::sync::Arc;
use anyhow::Result;
use tracing::{debug, trace};

use market_data_engine::handlers::EventProcessor;
use market_data_engine::types::{TradeV2, QuoteV2, MarketEvent};
use hft_event_bus::typed_bus::TypedEventBus;

/// Bridge for forwarding market data from exchange connections
/// to the market-data-engine for zero-allocation processing
/// and optionally to the event bus for other components.
pub struct MarketDataBridge {
    event_processor: Arc<EventProcessor>,
    event_bus: Arc<TypedEventBus>,
}

impl MarketDataBridge {
    /// Create a new market data bridge
    pub fn new(event_processor: Arc<EventProcessor>, event_bus: Arc<TypedEventBus>) -> Self {
        Self {
            event_processor,
            event_bus,
        }
    }
    
    /// Process a trade event with zero-allocation forwarding
    pub fn process_trade(&self, trade: TradeV2) -> Result<()> {
        trace!("Processing trade: {:?}", trade);
        
        // Forward to event bus for other components
        self.event_bus.publish(trade)?;
        
        debug!("Successfully processed and forwarded trade");
        Ok(())
    }
    
    /// Process a quote event with zero-allocation forwarding
    pub fn process_quote(&self, quote: QuoteV2) -> Result<()> {
        trace!("Processing quote: {:?}", quote);
        
        // Forward to event bus for other components
        self.event_bus.publish(quote)?;
        
        debug!("Successfully processed and forwarded quote");
        Ok(())
    }
    
    /// Get a reference to the event processor for direct integration
    pub fn event_processor(&self) -> Arc<EventProcessor> {
        self.event_processor.clone()
    }
    
    /// Get a reference to the event bus
    pub fn event_bus(&self) -> Arc<TypedEventBus> {
        self.event_bus.clone()
    }
    
    /// Get processing statistics
    pub fn get_stats(&self) -> MarketDataStats {
        // Get stats from event processor
        let processor_stats = self.event_processor.stats();
        
        MarketDataStats {
            trades_processed: processor_stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed),
            quotes_processed: processor_stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed),
            total_events: processor_stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed) 
                + processor_stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed),
            uptime: processor_stats.uptime(),
        }
    }
}

/// Market data processing statistics
#[derive(Debug, Clone)]
pub struct MarketDataStats {
    pub trades_processed: u64,
    pub quotes_processed: u64,
    pub total_events: u64,
    pub uptime: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use market_data_engine::types::{Price, Quantity, Timestamp, InstrumentId, Exchange};
    
    #[test]
    fn test_market_data_bridge_creation() {
        let event_bus = Arc::new(TypedEventBus::new());
        let event_processor = Arc::new(EventProcessor::warmup());
        
        let bridge = MarketDataBridge::new(event_processor, event_bus);
        
        // Verify bridge is created successfully
        let stats = bridge.get_stats();
        assert_eq!(stats.trades_processed, 0);
        assert_eq!(stats.quotes_processed, 0);
    }
    
    #[test]
    fn test_process_trade() -> Result<()> {
        let event_bus = Arc::new(TypedEventBus::new());
        let event_processor = Arc::new(EventProcessor::warmup());
        
        let bridge = MarketDataBridge::new(event_processor, event_bus);
        
        // Create a test trade
        let trade = TradeV2 {
            instrument_id: InstrumentId::new(Exchange::CME, "ES"),
            price: Price::from(4500.25),
            quantity: Quantity::from(1),
            timestamp: Timestamp::from(1234567890),
            exchange_timestamp: Some(Timestamp::from(1234567890)),
        };
        
        // Process the trade
        bridge.process_trade(trade)?;
        
        // Verify stats updated
        let stats = bridge.get_stats();
        assert_eq!(stats.trades_processed, 1);
        
        Ok(())
    }
    
    #[test]
    fn test_process_quote() -> Result<()> {
        let event_bus = Arc::new(TypedEventBus::new());
        let event_processor = Arc::new(EventProcessor::warmup());
        
        let bridge = MarketDataBridge::new(event_processor, event_bus);
        
        // Create a test quote
        let quote = QuoteV2 {
            instrument_id: InstrumentId::new(Exchange::CME, "ES"),
            bid_price: Price::from(4500.25),
            ask_price: Price::from(4500.50),
            bid_quantity: Quantity::from(10),
            ask_quantity: Quantity::from(5),
            timestamp: Timestamp::from(1234567890),
            exchange_timestamp: Some(Timestamp::from(1234567890)),
        };
        
        // Process the quote
        bridge.process_quote(quote)?;
        
        // Verify stats updated
        let stats = bridge.get_stats();
        assert_eq!(stats.quotes_processed, 1);
        
        Ok(())
    }
}
