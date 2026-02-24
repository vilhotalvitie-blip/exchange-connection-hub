//! Adapters for integrating with external systems

pub mod market_data;

#[cfg(feature = "event_bus")]
pub mod event_bus;

pub use market_data::{MarketDataBridge, MarketDataStats};
