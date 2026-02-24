//! Integration tests for exchange connection hub

use exchange_connection_hub::{ExchangeHub, ExchangeConfig};
use hft_event_bus::typed_bus::TypedEventBus;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_exchange_hub_creation() {
    let event_bus = Arc::new(TypedEventBus::new());
    let hub = ExchangeHub::new(event_bus);
    
    // Verify hub is created successfully
    assert_eq!(hub.connection_status().len(), 0);
    
    // Check event processor
    let event_processor = hub.event_processor();
    let stats = event_processor.get_stats();
    assert_eq!(stats.trades_processed.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(stats.quotes_processed.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[tokio::test]
async fn test_exchange_config_creation() {
    // Test NinjaTrader config
    let nt_config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
    assert_eq!(nt_config.connection_string, "127.0.0.1:7496");
    assert_eq!(nt_config.reconnect_attempts, 3);
    assert_eq!(nt_config.reconnect_delay_ms, 5000);
    
    // Test Interactive Brokers config
    let ib_config = ExchangeConfig::interactive_brokers("127.0.0.1:7497", 1);
    assert_eq!(ib_config.connection_string, "127.0.0.1:7497");
    assert_eq!(ib_config.settings.get("client_id"), Some(&serde_json::Value::Number(1.into())));
    
    // Test WebSocket config
    let ws_config = ExchangeConfig::websocket("binance", "wss://stream.binance.com:9443");
    assert_eq!(ws_config.connection_string, "wss://stream.binance.com:9443");
    
    // Test FIX config
    let fix_config = ExchangeConfig::fix("cme", "fix://cme.com:443");
    assert_eq!(fix_config.connection_string, "fix://cme.com:443");
}

#[tokio::test]
async fn test_market_data_bridge() {
    use exchange_connection_hub::adapters::market_data::MarketDataBridge;
    use market_data_engine::types::{TradeV2, QuoteV2, Price, Quantity, Timestamp, InstrumentId, Exchange};
    
    let event_bus = Arc::new(TypedEventBus::new());
    let event_processor = Arc::new(market_data_engine::handlers::EventProcessor::warmup());
    
    let bridge = MarketDataBridge::new(event_processor, event_bus);
    
    // Create test trade
    let trade = TradeV2 {
        instrument_id: InstrumentId::new(Exchange::CME, "ES"),
        price: Price::from(4500.25),
        quantity: Quantity::from(1),
        timestamp: Timestamp::from(1234567890),
        exchange_timestamp: Some(Timestamp::from(1234567890)),
    };
    
    // Process trade
    bridge.process_trade(trade).unwrap();
    
    // Check stats
    let stats = bridge.get_stats();
    assert_eq!(stats.trades_processed, 1);
    assert_eq!(stats.quotes_processed, 0);
    
    // Create test quote
    let quote = QuoteV2 {
        instrument_id: InstrumentId::new(Exchange::CME, "ES"),
        bid_price: Price::from(4500.25),
        ask_price: Price::from(4500.50),
        bid_quantity: Quantity::from(10),
        ask_quantity: Quantity::from(5),
        timestamp: Timestamp::from(1234567890),
        exchange_timestamp: Some(Timestamp::from(1234567890)),
    };
    
    // Process quote
    bridge.process_quote(quote).unwrap();
    
    // Check stats
    let stats = bridge.get_stats();
    assert_eq!(stats.trades_processed, 1);
    assert_eq!(stats.quotes_processed, 1);
}

#[tokio::test]
async fn test_ninjatrader_protocol_parsing() {
    use exchange_connection_hub::protocols::ninjatrader::NinjaTraderConnection;
    
    let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
    let connection = NinjaTraderConnection::new(config);
    
    // Test trade message parsing
    let mut trade_message = vec![0x01]; // Trade message type
    trade_message.extend_from_slice(b"ES\0\0\0\0\0"); // Symbol
    trade_message.extend_from_slice(&4500.25f64.to_le_bytes()); // Price
    trade_message.extend_from_slice(&1u32.to_le_bytes()); // Quantity
    
    let result = connection.parse_message(&trade_message);
    assert!(result.is_ok());
    
    if let Ok(exchange_connection_hub::types::ExchangeData::Trade(trade)) = result {
        assert_eq!(trade.price, Price::from(4500.25));
        assert_eq!(trade.quantity, Quantity::from(1));
    }
    
    // Test quote message parsing
    let mut quote_message = vec![0x02]; // Quote message type
    quote_message.extend_from_slice(b"ES\0\0\0\0\0"); // Symbol
    quote_message.extend_from_slice(&4500.25f64.to_le_bytes()); // Bid price
    quote_message.extend_from_slice(&4500.50f64.to_le_bytes()); // Ask price
    quote_message.extend_from_slice(&10u32.to_le_bytes()); // Bid quantity
    quote_message.extend_from_slice(&5u32.to_le_bytes()); // Ask quantity
    
    let result = connection.parse_message(&quote_message);
    assert!(result.is_ok());
    
    if let Ok(exchange_connection_hub::types::ExchangeData::Quote(quote)) = result {
        assert_eq!(quote.bid_price, Price::from(4500.25));
        assert_eq!(quote.ask_price, Price::from(4500.50));
        assert_eq!(quote.bid_quantity, Quantity::from(10));
        assert_eq!(quote.ask_quantity, Quantity::from(5));
    }
}

#[tokio::test]
async fn test_websocket_protocol_parsing() {
    use exchange_connection_hub::protocols::websocket::WebSocketConnection;
    
    let config = ExchangeConfig::websocket("test", "ws://localhost:8080");
    let connection = WebSocketConnection::new(config, "test".to_string());
    
    // Test trade JSON parsing
    let trade_json = r#"
    {
        "type": "trade",
        "symbol": "BTCUSDT",
        "price": 45000.5,
        "quantity": 100000000
    }
    "#;
    
    let result = connection.parse_json_message(trade_json);
    assert!(result.is_ok());
    
    if let Ok(exchange_connection_hub::types::ExchangeData::Trade(trade)) = result {
        assert_eq!(trade.price, Price::from(45000.5));
        assert_eq!(trade.quantity, Quantity::from(100000000));
    }
    
    // Test quote JSON parsing
    let quote_json = r#"
    {
        "type": "quote",
        "symbol": "BTCUSDT",
        "bid_price": 45000.0,
        "ask_price": 45001.0,
        "bid_quantity": 100000000,
        "ask_quantity": 50000000
    }
    "#;
    
    let result = connection.parse_json_message(quote_json);
    assert!(result.is_ok());
    
    if let Ok(exchange_connection_hub::types::ExchangeData::Quote(quote)) = result {
        assert_eq!(quote.bid_price, Price::from(45000.0));
        assert_eq!(quote.ask_price, Price::from(45001.0));
        assert_eq!(quote.bid_quantity, Quantity::from(100000000));
        assert_eq!(quote.ask_quantity, Quantity::from(50000000));
    }
}

#[tokio::test]
async fn test_exchange_registry() {
    use exchange_connection_hub::registry::ExchangeRegistry;
    use exchange_connection_hub::protocols::base::{ConnectionFactory, ExchangeFeature};
    use exchange_connection_hub::types::ExchangeId;
    
    let registry = ExchangeRegistry::new();
    
    // Check if NinjaTrader is registered (if feature is enabled)
    #[cfg(feature = "ninjatrader")]
    {
        assert!(registry.is_supported(ExchangeId::NinjaTrader));
        let features = registry.get_supported_features(ExchangeId::NinjaTrader);
        assert!(features.contains(&ExchangeFeature::MarketData));
    }
    
    // Check if WebSocket is registered (if feature is enabled)
    #[cfg(feature = "websocket")]
    {
        assert!(registry.is_supported(ExchangeId::WebSocket("generic".to_string())));
        let features = registry.get_supported_features(ExchangeId::WebSocket("generic".to_string()));
        assert!(features.contains(&ExchangeFeature::MarketData));
    }
    
    // List all exchanges
    let exchanges = registry.list_exchanges();
    println!("Registered exchanges: {:?}", exchanges);
    
    // Get exchange info
    let info = registry.get_exchange_info();
    assert!(!info.is_empty());
}

#[tokio::test]
async fn test_connection_manager() {
    use exchange_connection_hub::manager::ExchangeConnectionManager;
    use exchange_connection_hub::types::{ExchangeId, ConnectionStatus};
    
    let mut manager = ExchangeConnectionManager::new();
    
    // Initially no connections
    assert_eq!(manager.connection_status().len(), 0);
    
    // Test getting status for non-existent exchange
    let status = manager.get_connection_status(ExchangeId::NinjaTrader);
    assert!(status.is_none());
    
    // Test removing non-existent connection (should not panic)
    let result = manager.remove_connection(ExchangeId::NinjaTrader).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_end_to_end_data_flow() {
    // This test demonstrates the complete data flow:
    // Exchange Data -> Market Data Bridge -> Event Processor -> Event Bus
    
    use exchange_connection_hub::adapters::market_data::MarketDataBridge;
    use market_data_engine::types::{TradeV2, QuoteV2, Price, Quantity, Timestamp, InstrumentId, Exchange, MarketEvent};
    
    let event_bus = Arc::new(TypedEventBus::new());
    let event_processor = Arc::new(market_data_engine::handlers::EventProcessor::warmup());
    
    // Subscribe to events to verify they're published
    let event_bus_clone = event_bus.clone();
    let received_events = Arc::new(std::sync::Mutex::new(Vec::<MarketEvent>::new()));
    let received_events_clone = received_events.clone();
    
    tokio::spawn(async move {
        let mut receiver = event_bus_clone.subscribe::<MarketEvent>();
        while let Ok(event) = receiver.recv().await {
            received_events_clone.lock().unwrap().push(event);
        }
    });
    
    let bridge = MarketDataBridge::new(event_processor, event_bus);
    
    // Process a trade
    let trade = TradeV2 {
        instrument_id: InstrumentId::new(Exchange::CME, "ES"),
        price: Price::from(4500.25),
        quantity: Quantity::from(1),
        timestamp: Timestamp::from(1234567890),
        exchange_timestamp: Some(Timestamp::from(1234567890)),
    };
    
    bridge.process_trade(trade).unwrap();
    
    // Process a quote
    let quote = QuoteV2 {
        instrument_id: InstrumentId::new(Exchange::CME, "ES"),
        bid_price: Price::from(4500.25),
        ask_price: Price::from(4500.50),
        bid_quantity: Quantity::from(10),
        ask_quantity: Quantity::from(5),
        timestamp: Timestamp::from(1234567890),
        exchange_timestamp: Some(Timestamp::from(1234567890)),
    };
    
    bridge.process_quote(quote).unwrap();
    
    // Give some time for async processing
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Verify events were received
    let events = received_events.lock().unwrap();
    assert_eq!(events.len(), 2);
    
    match &events[0] {
        MarketEvent::Trade(t) => {
            assert_eq!(t.price, Price::from(4500.25));
            assert_eq!(t.quantity, Quantity::from(1));
        }
        _ => panic!("Expected trade event"),
    }
    
    match &events[1] {
        MarketEvent::Quote(q) => {
            assert_eq!(q.bid_price, Price::from(4500.25));
            assert_eq!(q.ask_price, Price::from(4500.50));
        }
        _ => panic!("Expected quote event"),
    }
    
    // Verify bridge statistics
    let stats = bridge.get_stats();
    assert_eq!(stats.trades_processed, 1);
    assert_eq!(stats.quotes_processed, 1);
    assert_eq!(stats.total_events, 2);
}
