//! NinjaTrader protocol implementation

use async_trait::async_trait;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::Read;
use tracing::{info, warn, error, debug};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::protocols::base::{ExchangeConnection, BaseExchangeConnection, ConnectionFactory, ExchangeFeature};
use crate::types::{ExchangeId, ExchangeConfig, Symbol, ConnectionHealth, OrderRequest, OrderId, ExchangeData};
use market_data_engine::types::{TradeV2, QuoteV2, OrderBookSnapshot, Price, Quantity, Timestamp, InstrumentId, Exchange, SideV2, TradeFlags};

/// NinjaTrader connection implementation
pub struct NinjaTraderConnection {
    base: BaseExchangeConnection,
    listener: Option<TcpListener>,
    active_connection: Option<TcpStream>,
}

impl NinjaTraderConnection {
    /// Create a new NinjaTrader connection
    pub fn new(config: ExchangeConfig) -> Self {
        Self {
            base: BaseExchangeConnection::new(ExchangeId::NinjaTrader, config).expect("Failed to create base connection"),
            listener: None,
            active_connection: None,
        }
    }
    
    /// Parse NinjaTrader binary protocol message
    fn parse_message(&self, data: &[u8]) -> Result<ExchangeData> {
        if data.len() < 4 {
            return Err(anyhow::anyhow!("Message too short"));
        }
        
        // Read message type (first byte)
        let message_type = data[0];
        
        match message_type {
            0x01 => self.parse_trade(&data[1..]),
            0x02 => self.parse_quote(&data[1..]),
            0x03 => self.parse_order_book(&data[1..]),
            _ => Err(anyhow::anyhow!("Unknown message type: 0x{:02x}", message_type)),
        }
    }
    
    /// Parse trade message
    fn parse_trade(&self, data: &[u8]) -> Result<ExchangeData> {
        if data.len() < 20 {
            return Err(anyhow::anyhow!("Trade message too short"));
        }
        
        let mut cursor = std::io::Cursor::new(data);
        
        // Read symbol (8 bytes)
        let mut symbol_bytes = [0u8; 8];
                        Read::read_exact(&mut cursor, &mut symbol_bytes)?;
        let symbol = std::str::from_utf8(&symbol_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid symbol"))?
            .trim_end_matches('\0');
        
        // Read price (8 bytes, little-endian double)
        let price = ReadBytesExt::read_f64::<LittleEndian>(&mut cursor)?;
        
        // Read quantity (4 bytes, little-endian u32)
        let quantity = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor)?;
        
        // Create trade
        let trade = TradeV2 {
            instrument_id: InstrumentId::new(Exchange::Unknown, symbol),
            price: Price::from_float(price),
            quantity: Quantity::from(quantity),
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
    
    /// Parse quote message
    pub fn parse_quote(&self, data: &[u8]) -> Result<ExchangeData> {
        if data.len() < 36 {
            return Err(anyhow::anyhow!("Quote message too short"));
        }
        
        let mut cursor = std::io::Cursor::new(data);
        
        // Read symbol (8 bytes)
        let mut symbol_bytes = [0u8; 8];
                        Read::read_exact(&mut cursor, &mut symbol_bytes)?;
        let symbol = std::str::from_utf8(&symbol_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid symbol"))?
            .trim_end_matches('\0');
        
        // Read bid price (8 bytes, little-endian double)
        let bid_price = ReadBytesExt::read_f64::<LittleEndian>(&mut cursor).unwrap_or(0.0);
        
        // Read ask price (8 bytes, little-endian double)
        let ask_price = ReadBytesExt::read_f64::<LittleEndian>(&mut cursor).unwrap_or(0.0);
        
        // Read bid quantity (4 bytes, little-endian u32)
        let bid_quantity = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap_or(0);
        
        // Read ask quantity (4 bytes, little-endian u32)
        let ask_quantity = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap_or(0);
        
        // Create quote
        let quote = QuoteV2 {
            instrument_id: InstrumentId::new(Exchange::Unknown, symbol),
            bid_price: Price::from_float(bid_price),
            ask_price: Price::from_float(ask_price),
            bid_size: Quantity::from(bid_quantity),
            ask_size: Quantity::from(ask_quantity),
            timestamp: Timestamp::from_nanos(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64),
            exchange: 0, // TODO: proper exchange mapping
            _padding: [0; 14],
        };
        
        Ok(ExchangeData::Quote(quote))
    }
    
    /// Parse order book message
    fn parse_order_book(&self, _data: &[u8]) -> Result<ExchangeData> {
        // Simplified order book parsing
        // In a real implementation, this would parse the full order book structure
        warn!("Order book parsing not fully implemented");
        
        // Return empty order book for now
        let order_book = OrderBookSnapshot {
            instrument_id: 1, // TODO: proper instrument ID mapping
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64,
            sequence: 0, // TODO: proper sequence numbering
        };
        
        Ok(ExchangeData::OrderBook(order_book))
    }
    
    /// Handle incoming connection
    async fn handle_connection(&mut self, mut socket: TcpStream) -> Result<()> {
        info!("NinjaTrader client connected");
        
        let mut buffer = vec![0u8; 65536];
        let mut message_buffer = Vec::with_capacity(131072);
        
        loop {
            let n = socket.read(&mut buffer).await
                .context("Failed to read from socket")?;
            
            if n == 0 {
                info!("NinjaTrader client disconnected");
                break;
            }
            
            message_buffer.extend_from_slice(&buffer[..n]);
            
            // Process length-prefixed messages
            while message_buffer.len() >= 4 {
                let len = u32::from_le_bytes([
                    message_buffer[0],
                    message_buffer[1],
                    message_buffer[2],
                    message_buffer[3],
                ]) as usize;
                
                if message_buffer.len() < 4 + len {
                    break;
                }
                
                // Parse message
                match self.parse_message(&message_buffer[4..4+len]) {
                    Ok(data) => {
                        self.base.update_received_stats(len);
                        self.base.send_data(data)?;
                    }
                    Err(e) => {
                        warn!("Failed to parse NinjaTrader message: {}", e);
                        self.base.increment_error_count();
                    }
                }
                
                message_buffer.drain(..4+len);
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for NinjaTraderConnection {
    async fn connect(&mut self) -> Result<()> {
        let addr = self.base.config().connection_string.clone();
        info!("Binding NinjaTrader server to {}", addr);
        
        let listener = TcpListener::bind(&addr).await
            .with_context(|| format!("Failed to bind to {}", addr))?;
        
        self.listener = Some(listener);
        self.base.set_connected(true);
        
        info!("NinjaTrader server listening on {}", addr);
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<()> {
        self.base.set_connected(false);
        self.listener = None;
        self.active_connection = None;
        info!("NinjaTrader server disconnected");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }
    
    async fn health_check(&self) -> Result<ConnectionHealth> {
        if self.is_connected() {
            Ok(ConnectionHealth::Healthy)
        } else {
            Ok(ConnectionHealth::Disconnected)
        }
    }
    
    async fn subscribe_market_data(&mut self, _symbols: &[Symbol]) -> Result<()> {
        // NinjaTrader doesn't have explicit subscription - it sends all data
        info!("NinjaTrader sends all market data by default");
        Ok(())
    }
    
    async fn unsubscribe_market_data(&mut self, _symbols: &[Symbol]) -> Result<()> {
        // No unsubscribe for NinjaTrader
        Ok(())
    }
    
    async fn submit_order(&mut self, _order: OrderRequest) -> Result<OrderId> {
        Err(anyhow::anyhow!("Order execution not supported for NinjaTrader"))
    }
    
    async fn cancel_order(&mut self, _order_id: OrderId) -> Result<()> {
        Err(anyhow::anyhow!("Order execution not supported for NinjaTrader"))
    }
    
    fn exchange_id(&self) -> ExchangeId {
        self.base.exchange_id()
    }
    
    fn get_stats(&self) -> crate::protocols::base::ConnectionStats {
        self.base.get_stats()
    }
}

/// Factory for creating NinjaTrader connections
pub struct NinjaTraderFactory;

impl NinjaTraderFactory {
    pub fn new() -> Self {
        Self
    }
}

impl ConnectionFactory for NinjaTraderFactory {
    fn create_connection(&self, config: ExchangeConfig) -> Result<Arc<dyn ExchangeConnection>> {
        Ok(Arc::new(NinjaTraderConnection::new(config)))
    }
    
    fn exchange_id(&self) -> ExchangeId {
        ExchangeId::NinjaTrader
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
    fn test_parse_trade() {
        let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
        let connection = NinjaTraderConnection::new(config);
        
        // Create a test trade message
        let mut message = vec![0x01]; // Trade message type
        message.extend_from_slice(b"ES\0\0\0\0\0"); // Symbol
        message.extend_from_slice(&4500.25f64.to_le_bytes()); // Price
        message.extend_from_slice(&1u32.to_le_bytes()); // Quantity
        
        let result = connection.parse_message(&message);
        assert!(result.is_ok());
        
        if let Ok(ExchangeData::Trade(trade)) = result {
            assert_eq!(trade.price, Price::from(4500.25));
            assert_eq!(trade.quantity, Quantity::from(1));
        }
    }
    
    #[test]
    fn test_parse_quote() {
        let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
        let connection = NinjaTraderConnection::new(config);
        
        // Create a test quote message
        let mut message = vec![0x02]; // Quote message type
        message.extend_from_slice(b"ES\0\0\0\0\0"); // Symbol
        message.extend_from_slice(&4500.25f64.to_le_bytes()); // Bid price
        message.extend_from_slice(&4500.50f64.to_le_bytes()); // Ask price
        message.extend_from_slice(&10u32.to_le_bytes()); // Bid quantity
        message.extend_from_slice(&5u32.to_le_bytes()); // Ask quantity
        
        let result = connection.parse_message(&message);
        assert!(result.is_ok());
        
        if let Ok(ExchangeData::Quote(quote)) = result {
            assert_eq!(quote.bid_price, Price::from(4500.25));
            assert_eq!(quote.ask_price, Price::from(4500.50));
            assert_eq!(quote.bid_quantity, Quantity::from(10));
            assert_eq!(quote.ask_quantity, Quantity::from(5));
        }
    }
}
