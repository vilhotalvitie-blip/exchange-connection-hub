//! Example binary for exchange connection hub

use exchange_connection_hub::{ExchangeHub, ExchangeConfig};
use hft_event_bus::typed_bus::TypedEventBus;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create event bus
    let event_bus = Arc::new(TypedEventBus::new());
    
    // Create exchange hub
    let mut hub = ExchangeHub::new(event_bus);
    
    println!("🚀 Exchange Connection Hub - Example");
    println!("📡 Available exchanges:");
    println!("  - NinjaTrader (TCP)");
    println!("  - WebSocket (JSON)");
    
    // For now, just show the hub is created
    println!("✅ Exchange hub created successfully");
    println!("📊 Connection status: {} connections", hub.connection_status().len());
    
    // Show statistics
    let event_processor = hub.event_processor();
    let stats = event_processor.get_stats();
    println!("📈 Initial statistics:");
    println!("  Trades processed: {}", stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Quotes processed: {}", stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Uptime: {:?}", stats.uptime());
    
    println!("🎯 Example hub is ready. Press Ctrl+C to stop.");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    println!("👋 Shutting down example");
    
    Ok(())
}
