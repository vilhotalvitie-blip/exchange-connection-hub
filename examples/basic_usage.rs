//! Basic usage example for exchange connection hub

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
    
    println!("🚀 Starting Exchange Connection Hub");
    
    // Connect to NinjaTrader
    println!("📡 Connecting to NinjaTrader...");
    let nt_config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
    match hub.connect_exchange(exchange_connection_hub::types::ExchangeId::NinjaTrader, nt_config).await {
        Ok(()) => println!("✅ NinjaTrader connected successfully"),
        Err(e) => println!("❌ Failed to connect to NinjaTrader: {}", e),
    }
    
    // Subscribe to market data
    println!("📊 Subscribing to market data...");
    match hub.subscribe_market_data(
        exchange_connection_hub::types::ExchangeId::NinjaTrader, 
        &["ES", "NQ"]
    ).await {
        Ok(()) => println!("✅ Subscribed to ES and NQ"),
        Err(e) => println!("❌ Failed to subscribe: {}", e),
    }
    
    // Show connection status
    println!("📈 Connection Status:");
    for (exchange_id, is_healthy) in hub.connection_status() {
        println!("  {:?}: {}", exchange_id, if is_healthy { "✅ Healthy" } else { "❌ Unhealthy" });
    }
    
    // Show statistics
    println!("📊 Statistics:");
    let event_processor = hub.event_processor();
    let stats = event_processor.get_stats();
    println!("  Trades processed: {}", stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Quotes processed: {}", stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Uptime: {:?}", stats.uptime());
    
    println!("🎯 Exchange hub is running. Press Ctrl+C to stop...");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    println!("👋 Shutting down Exchange Connection Hub");
    
    Ok(())
}
