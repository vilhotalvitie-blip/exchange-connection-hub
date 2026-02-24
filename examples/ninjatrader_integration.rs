//! NinjaTrader integration example

use exchange_connection_hub::{ExchangeHub, ExchangeConfig};
use hft_event_bus::typed_bus::TypedEventBus;
use std::sync::Arc;
use market_data_engine::types::MarketEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔌 NinjaTrader Integration Example");
    
    // Create event bus and subscribe to events
    let event_bus = Arc::new(TypedEventBus::new());
    
    // Subscribe to market events for demonstration
    let event_bus_clone = event_bus.clone();
    tokio::spawn(async move {
        let mut receiver = event_bus_clone.subscribe::<MarketEvent>();
        while let Ok(event) = receiver.recv().await {
            match event {
                MarketEvent::Trade(trade) => {
                    println!("📈 Trade: {} @ {} (Qty: {})", 
                            trade.instrument_id, trade.price, trade.quantity);
                }
                MarketEvent::Quote(quote) => {
                    println!("💰 Quote: {} Bid: {}@{} Ask: {}@{}",
                            quote.instrument_id,
                            quote.bid_quantity, quote.bid_price,
                            quote.ask_quantity, quote.ask_price);
                }
                MarketEvent::OrderBook(ob) => {
                    println!("📊 OrderBook: {} ({} bids, {} asks)",
                            ob.instrument_id, ob.bids.len(), ob.asks.len());
                }
                _ => {}
            }
        }
    });
    
    // Create exchange hub
    let mut hub = ExchangeHub::new(event_bus);
    
    // Configure NinjaTrader connection
    let mut nt_config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
    nt_config.reconnect_attempts = 5;
    nt_config.reconnect_delay_ms = 3000;
    
    println!("🔗 Connecting to NinjaTrader at {}", nt_config.connection_string);
    
    // Connect to NinjaTrader
    match hub.connect_exchange(exchange_connection_hub::types::ExchangeId::NinjaTrader, nt_config).await {
        Ok(()) => {
            println!("✅ Successfully connected to NinjaTrader");
            
            // Subscribe to market data
            let symbols = vec!["ES", "NQ", "YM", "RTY"];
            println!("📊 Subscribing to symbols: {:?}", symbols);
            
            match hub.subscribe_market_data(
                exchange_connection_hub::types::ExchangeId::NinjaTrader, 
                &symbols.iter().map(|s| s.as_ref()).collect::<Vec<_>>()
            ).await {
                Ok(()) => println!("✅ Successfully subscribed to market data"),
                Err(e) => println!("⚠️  Subscription warning: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to NinjaTrader: {}", e);
            println!("💡 Make sure NinjaTrader is running and configured to send data to 127.0.0.1:7496");
            return Ok(());
        }
    }
    
    // Monitor connection status
    let hub_clone = hub.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            println!("📈 Connection Status:");
            for (exchange_id, is_healthy) in hub_clone.connection_status() {
                println!("  {:?}: {}", exchange_id, if is_healthy { "✅ Healthy" } else { "❌ Unhealthy" });
            }
            
            // Show statistics
            let event_processor = hub_clone.event_processor();
            let stats = event_processor.get_stats();
            println!("📊 Statistics:");
            println!("  Trades processed: {}", stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed));
            println!("  Quotes processed: {}", stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed));
            println!("  Uptime: {:?}", stats.uptime());
            println!();
        }
    });
    
    println!("🎯 NinjaTrader integration is running. Press Ctrl+C to stop...");
    println!("💡 Start sending data from NinjaTrader to see events printed above");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    println!("👋 Shutting down NinjaTrader integration");
    
    Ok(())
}
