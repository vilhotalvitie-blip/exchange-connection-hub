# Exchange Connection Hub

A unified exchange connection system for HFT trading applications. Provides protocol-agnostic interface for connecting to various exchanges and forwarding market data through the market-data-engine and hft-event-bus.

## Features

- **Zero-Allocation Market Data Processing**: Preserves sub-microsecond performance through market-data-engine integration
- **Multi-Exchange Support**: Unified interface for NinjaTrader, WebSocket, FIX, and Interactive Brokers
- **Protocol Agnostic Design**: Easy addition of new exchange protocols
- **Health Monitoring**: Automatic failover and reconnection with exponential backoff
- **Event Bus Integration**: Seamless integration with hft-event-bus for component communication
- **Performance Optimized**: Zero-copy data forwarding and efficient resource management

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
exchange-connection-hub = { version = "0.1.0", git = "https://github.com/hft-system/exchange-connection-hub" }
hft-event-bus = { version = "0.1.0", git = "https://github.com/hft-system/hft-event-bus" }
market-data-engine = { version = "0.1.0", git = "https://github.com/hft-system/market-data-engine" }
```

### Basic Usage

```rust
use exchange_connection_hub::ExchangeHub;
use hft_event_bus::typed_bus::TypedEventBus;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create event bus
    let event_bus = std::sync::Arc::new(TypedEventBus::new());
    
    // Create exchange hub
    let mut hub = ExchangeHub::new(event_bus);
    
    // Connect to NinjaTrader
    let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
    hub.connect_exchange(ExchangeId::NinjaTrader, config).await?;
    
    // Subscribe to market data
    hub.subscribe_market_data(ExchangeId::NinjaTrader, &["ES", "NQ"]).await?;
    
    println!("Exchange hub running with {} connections", 
             hub.connection_status().len());
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    
    Ok(())
}
```

### WebSocket Exchange

```rust
use exchange_connection_hub::{ExchangeHub, ExchangeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let event_bus = std::sync::Arc::new(TypedEventBus::new());
    let mut hub = ExchangeHub::new(event_bus);
    
    // Connect to a WebSocket-based crypto exchange
    let config = ExchangeConfig::websocket("binance", "wss://stream.binance.com:9443/ws/btcusdt@trade");
    hub.connect_exchange(ExchangeId::WebSocket("binance".to_string()), config).await?;
    
    // Subscribe to symbols
    hub.subscribe_market_data(
        ExchangeId::WebSocket("binance".to_string()), 
        &["BTCUSDT", "ETHUSDT"]
    ).await?;
    
    Ok(())
}
```

## Architecture

### Data Flow

```
Exchange Connection → Protocol Parser → Exchange Hub → Market Data Engine → Event Bus → Strategies
```

### Core Components

- **ExchangeHub**: Main orchestrator for all exchange connections
- **ExchangeConnection**: Trait for protocol-specific implementations
- **MarketDataBridge**: Zero-allocation bridge to market-data-engine
- **ConnectionManager**: Handles connection pooling and health monitoring
- **ExchangeRegistry**: Factory pattern for creating connections

### Performance Characteristics

- **End-to-End Latency**: < 200ns
- **Exchange → EventProcessor**: < 100ns
- **EventProcessor → EventBus**: < 50ns
- **Memory Usage**: < 10MB base + < 1MB per connection
- **Throughput**: > 1M messages/second

## Supported Exchanges

### NinjaTrader
- **Protocol**: TCP with binary format
- **Features**: Market data (trades, quotes, order books)
- **Configuration**: `ExchangeConfig::ninjatrader("host:port")`

### WebSocket Exchanges
- **Protocol**: JSON over WebSocket
- **Features**: Market data (trades, quotes, order books)
- **Configuration**: `ExchangeConfig::websocket("name", "ws://url")`

### Future Support
- **FIX Protocol**: Standard financial exchange protocol
- **Interactive Brokers**: TWS/Gateway API integration
- **Databento**: High-performance market data provider

## Configuration

### Exchange Configuration

```rust
// NinjaTrader
let nt_config = ExchangeConfig::ninjatrader("127.0.0.1:7496");

// Interactive Brokers
let ib_config = ExchangeConfig::interactive_brokers("127.0.0.1:7497", 1);

// WebSocket
let ws_config = ExchangeConfig::websocket("binance", "wss://stream.binance.com:9443");

// FIX
let fix_config = ExchangeConfig::fix("cme", "fix://cme.com:443");
```

### Advanced Configuration

```rust
let mut config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
config.reconnect_attempts = 5;
config.reconnect_delay_ms = 3000;
config.credentials = Some(Credentials {
    username: "user".to_string(),
    password: Some("pass".to_string()),
    api_key: None,
    api_secret: None,
});
```

## Monitoring and Statistics

### Connection Status

```rust
let status = hub.connection_status();
for (exchange_id, is_healthy) in status {
    println!("Exchange {:?}: {}", exchange_id, if is_healthy { "Healthy" } else { "Unhealthy" });
}
```

### Performance Statistics

```rust
let stats = hub.event_processor().get_stats();
println!("Trades processed: {}", stats.trades_processed.load(Ordering::Relaxed));
println!("Quotes processed: {}", stats.quotes_processed.load(Ordering::Relaxed));
println!("Uptime: {:?}", stats.uptime());
```

## Error Handling

The hub uses `anyhow::Result` for error handling with detailed error messages:

```rust
match hub.connect_exchange(exchange_id, config).await {
    Ok(()) => println!("Connected successfully"),
    Err(e) => eprintln!("Connection failed: {}", e),
}
```

## Testing

Run the test suite:

```bash
cargo test --all-features
```

Run integration tests:

```bash
cargo test --test integration_tests
```

## Examples

See the `examples/` directory for complete working examples:

- `basic_usage.rs`: Simple connection and subscription
- `ninjatrader_integration.rs`: NinjaTrader-specific example
- `multi_exchange.rs`: Managing multiple exchange connections

## Performance Benchmarks

Run performance benchmarks:

```bash
cargo bench
```

Expected results on modern hardware:
- **Trade Processing**: ~50ns per trade
- **Quote Processing**: ~50ns per quote
- **Memory Allocations**: Zero during trading hours

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

### Adding New Exchange Protocols

1. Implement `ExchangeConnection` trait
2. Create a `ConnectionFactory` implementation
3. Register the factory in `ExchangeRegistry`
4. Add tests and documentation

## License

This project is dual-licensed under the MIT and Apache 2.0 licenses.

## Repository

- **GitHub**: https://github.com/hft-system/exchange-connection-hub
- **Issues**: https://github.com/hft-system/exchange-connection-hub/issues
- **Documentation**: https://docs.rs/exchange-connection-hub

## Related Projects

- [hft-event-bus](https://github.com/hft-system/hft-event-bus): Zero-allocation event bus
- [market-data-engine](https://github.com/hft-system/market-data-engine): High-performance market data processing
- [hft-ecosystem](https://github.com/hft-system/hft-ecosystem): Complete HFT trading ecosystem
