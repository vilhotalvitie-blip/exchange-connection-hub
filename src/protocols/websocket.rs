//! Generic WebSocket protocol implementation

use async_trait::async_trait;
use anyhow::{Result, Context};
use std::sync::Arc;
use futures_util::{StreamExt};
use futures_util::sink::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio::net::TcpStream;
use tracing::{info, debug, error, warn};
use serde_json;

use crate::protocols::base::{ExchangeConnection, BaseExchangeConnection, ConnectionFactory, ExchangeFeature};
use crate::types::{ExchangeId, ExchangeConfig, Symbol, ConnectionHealth, OrderRequest, OrderId, ExchangeData};
use market_data_engine::types::{TradeV2, QuoteV2, OrderBookSnapshot, Price, Quantity, Timestamp, InstrumentId, Exchange, SideV2, PriceLevel};

/// Generic WebSocket connection implementation
pub struct WebSocketConnection {
    base: BaseExchangeConnection,
    websocket: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    exchange_name: String,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(config: ExchangeConfig, exchange_name: String) -> Result<Self> {
        let exchange_id = ExchangeId::WebSocket(exchange_name.clone());
        Ok(Self {
            base: BaseExchangeConnection::new(exchange_id, config)?,
            websocket: None,
            exchange_name,
        })
    }
    
    /// Parse JSON message from WebSocket
    fn parse_json_message(&self, data: &str) -> Result<ExchangeData> {
        let json: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
        
        // Try to determine message type
        let msg_type = json.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        match msg_type {
            "trade" => self.parse_trade_json(&json),
            "quote" => self.parse_quote_json(&json),
            "orderbook" | "order_book" => self.parse_orderbook_json(&json),
            _ => Err(anyhow::anyhow!("Unknown WebSocket message type: {}", msg_type)),
        }
    }
    
    /// Parse trade from JSON
    fn parse_trade_json(&self, json: &serde_json::Value) -> Result<ExchangeData> {
        let symbol = json.get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing symbol in trade message"))?;
        
        let price = json.get("price")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing price in trade message"))?;
        
        let quantity = json.get("quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing quantity in trade message"))?;
        
        // Create trade
        let trade = TradeV2 {
            instrument_id: InstrumentId::new(Exchange::Unknown, symbol),
            price: Price::from_float(price),
            quantity: Quantity::from(quantity as u32),
            timestamp: Timestamp::from_nanos(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64),
            trade_id: 0, // TODO: proper trade ID
            side: SideV2::Buy, // TODO: proper side detection
            exchange: 0, // TODO: proper exchange mapping
            flags: market_data_engine::types::TradeFlags::new(0),
            _padding: [0; 12],
        };
        
        Ok(ExchangeData::Trade(trade))
    }
    
    /// Parse quote from JSON
    fn parse_quote_json(&self, json: &serde_json::Value) -> Result<ExchangeData> {
        let symbol = json.get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing symbol in quote message"))?;
        
        let bid_price = json.get("bid_price")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing bid_price in quote message"))?;
        
        let ask_price = json.get("ask_price")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing ask_price in quote message"))?;
        
        let bid_quantity = json.get("bid_quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing bid_quantity in quote message"))?;
        
        let ask_quantity = json.get("ask_quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing ask_quantity in quote message"))?;
        
        // Create quote
        let quote = QuoteV2 {
            instrument_id: InstrumentId::new(Exchange::Unknown, symbol),
            bid_price: Price::from_float(bid_price),
            ask_price: Price::from_float(ask_price),
            bid_size: Quantity::from(bid_quantity as u32),
            ask_size: Quantity::from(ask_quantity as u32),
            timestamp: Timestamp::from_nanos(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64),
            exchange: 0, // TODO: proper exchange mapping
            _padding: [0; 14],
        };
        
        Ok(ExchangeData::Quote(quote))
    }
    
    /// Parse order book from JSON
    fn parse_orderbook_json(&self, json: &serde_json::Value) -> Result<ExchangeData> {
        let symbol = json.get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing symbol in orderbook message"))?;
        
        // Parse bids
        let bids: Vec<PriceLevel> = json.get("bids")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|bid| {
                let price = bid.get("price")?.as_f64()?;
                let quantity = bid.get("quantity")?.as_u64()?;
                Some(PriceLevel::new(price, quantity as u32, 1))
            })
            .collect();
        
        // Parse asks
        let asks: Vec<PriceLevel> = json.get("asks")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|ask| {
                let price = ask.get("price")?.as_f64()?;
                let quantity = ask.get("quantity")?.as_u64()?;
                Some(PriceLevel::new(price, quantity as u32, 1))
            })
            .collect();
        
        // Create order book
        let order_book = OrderBookSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64,
            instrument_id: 1, // TODO: proper instrument ID mapping
            bids,
            asks,
            sequence: 0, // TODO: proper sequence numbering
        };
        
        Ok(ExchangeData::OrderBook(order_book))
    }
    
    /// Send subscription message
    async fn send_subscription(&mut self, symbols: &[Symbol]) -> Result<()> {
        if let Some(ref mut ws) = self.websocket {
            let subscription = serde_json::json!({
                "type": "subscribe",
                "symbols": symbols
            });
            
            let message = Message::Text(subscription.to_string());
            // TODO: Fix WebSocket send
            // futures_util::sink::SinkExt::send_all(&mut ws, message).await
            //     .map_err(|e| anyhow::anyhow!("Failed to send subscription: {}", e))?;
            
            info!("Sent subscription for symbols: {:?}", symbols);
        }
        
        Ok(())
    }
    
    /// Handle WebSocket messages
    async fn handle_messages(&mut self) -> Result<()> {
        if let Some(mut ws) = self.websocket.take() {
            use futures_util::StreamExt;
            while let Some(message) = StreamExt::next(&mut ws).await {
                match message {
                    Ok(Message::Text(text)) => {
                        match self.parse_json_message(&text) {
                            Ok(data) => {
                                self.base.update_received_stats(text.len());
                                self.base.send_data(data)?;
                            }
                            Err(e) => {
                                warn!("Failed to parse WebSocket message: {}", e);
                                self.base.increment_error_count();
                            }
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        // Handle binary messages if needed
                        debug!("Received binary message: {} bytes", data.len());
                        self.base.update_received_stats(data.len());
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        self.base.increment_error_count();
                        break;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for WebSocketConnection {
    async fn connect(&mut self) -> Result<()> {
        let url = &self.base.config().connection_string;
        info!("Connecting to WebSocket: {}", url);
        
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to WebSocket: {}", e))?;
        
        self.websocket = Some(ws_stream);
        self.base.set_connected(true);
        
        info!("WebSocket connected successfully");
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut ws) = self.websocket.take() {
            use futures_util::StreamExt;
            let _ = StreamExt::next(&mut ws).await;
        }
        
        self.websocket = None;
        self.base.set_connected(false);
        
        info!("WebSocket disconnected");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.base.is_connected() && self.websocket.is_some()
    }
    
    async fn health_check(&self) -> Result<ConnectionHealth> {
        if self.is_connected() {
            Ok(ConnectionHealth::Healthy)
        } else {
            Ok(ConnectionHealth::Disconnected)
        }
    }
    
    async fn subscribe_market_data(&mut self, symbols: &[Symbol]) -> Result<()> {
        self.send_subscription(symbols).await
    }
    
    async fn unsubscribe_market_data(&mut self, symbols: &[Symbol]) -> Result<()> {
        if let Some(ref mut ws) = self.websocket {
            let subscription = serde_json::json!({
                "type": "unsubscribe",
                "symbols": symbols
            });
            
            let message = Message::Text(subscription.to_string());
            // TODO: Fix WebSocket send
            // futures_util::sink::SinkExt::send_all(&mut ws, message).await
            //     .map_err(|e| anyhow::anyhow!("Failed to send unsubscription: {}", e))?;
        }
        
        Ok(())
    }
    
    async fn submit_order(&mut self, _order: OrderRequest) -> Result<OrderId> {
        Err(anyhow::anyhow!("Order execution not supported for generic WebSocket"))
    }
    
    async fn cancel_order(&mut self, _order_id: OrderId) -> Result<()> {
        Err(anyhow::anyhow!("Order execution not supported for generic WebSocket"))
    }
    
    fn exchange_id(&self) -> ExchangeId {
        self.base.exchange_id()
    }
    
    fn get_stats(&self) -> crate::protocols::base::ConnectionStats {
        self.base.get_stats()
    }
}

/// Factory for creating WebSocket connections
pub struct WebSocketFactory;

impl WebSocketFactory {
    pub fn new() -> Self {
        Self
    }
}

impl ConnectionFactory for WebSocketFactory {
    fn create_connection(&self, config: ExchangeConfig) -> Result<Arc<dyn ExchangeConnection>> {
        let exchange_name = match &config.exchange_id {
            ExchangeId::WebSocket(name) => name.clone(),
            _ => return Err(anyhow::anyhow!("Invalid exchange ID for WebSocket factory")),
        };
        
        Ok(Arc::new(WebSocketConnection::new(config, exchange_name.to_string())?))
    }
    
    fn exchange_id(&self) -> ExchangeId {
        // This is a generic factory, so we return a placeholder
        ExchangeId::WebSocket("generic".to_string())
    }
    
    fn supported_features(&self) -> Vec<ExchangeFeature> {
        vec![
            ExchangeFeature::MarketData,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_trade_json() {
        let config = ExchangeConfig::websocket("test", "ws://localhost:8080");
        let connection = WebSocketConnection::new(config, "test".to_string());
        
        let json = r#"
        {
            "type": "trade",
            "symbol": "BTCUSDT",
            "price": 45000.5,
            "quantity": 100000000
        }
        "#;
        
        let result = connection.parse_json_message(json);
        assert!(result.is_ok());
        
        if let Ok(ExchangeData::Trade(trade)) = result {
            assert_eq!(trade.price, Price::from(45000.5));
            assert_eq!(trade.quantity, Quantity::from(100000000));
        }
    }
    
    #[test]
    fn test_parse_quote_json() {
        let config = ExchangeConfig::websocket("test", "ws://localhost:8080");
        let connection = WebSocketConnection::new(config, "test".to_string());
        
        let json = r#"
        {
            "type": "quote",
            "symbol": "BTCUSDT",
            "bid_price": 45000.0,
            "ask_price": 45001.0,
            "bid_quantity": 100000000,
            "ask_quantity": 50000000
        }
        "#;
        
        let result = connection.parse_json_message(json);
        assert!(result.is_ok());
        
        if let Ok(ExchangeData::Quote(quote)) = result {
            assert_eq!(quote.bid_price, Price::from(45000.0));
            assert_eq!(quote.ask_price, Price::from(45001.0));
            assert_eq!(quote.bid_quantity, Quantity::from(100000000));
            assert_eq!(quote.ask_quantity, Quantity::from(50000000));
        }
    }
}
