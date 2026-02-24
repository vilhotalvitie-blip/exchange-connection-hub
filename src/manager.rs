//! Connection manager for handling multiple exchange connections

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn, error, debug};

use crate::protocols::base::ExchangeConnection;
use crate::types::{ExchangeId, Symbol, ExchangeData, ConnectionStatus, ConnectionHealth};

/// Manager for multiple exchange connections with health monitoring
pub struct ExchangeConnectionManager {
    connections: Arc<RwLock<HashMap<ExchangeId, Arc<dyn ExchangeConnection>>>>,
    status: Arc<RwLock<HashMap<ExchangeId, ConnectionStatus>>>,
}

impl ExchangeConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a new connection to the manager
    pub async fn add_connection(&mut self, exchange_id: ExchangeId, connection: Arc<dyn ExchangeConnection>) -> Result<()> {
        info!("Adding connection for exchange: {:?}", exchange_id);
        
        // Store connection
        {
            let mut connections = self.connections.write().unwrap();
            connections.insert(exchange_id.clone(), connection.clone());
        }
        
        // Initialize status
        {
            let mut status = self.status.write().unwrap();
            status.insert(exchange_id.clone(), ConnectionStatus {
                exchange_id,
                health: ConnectionHealth::Healthy,
                last_message_time: None,
                messages_received: 0,
                bytes_received: 0,
                error_count: 0,
            });
        }
        
        // Start health monitoring
        self.start_health_monitoring(exchange_id.clone(), connection.clone()).await;
        
        info!("Successfully added connection for exchange: {:?}", exchange_id);
        Ok(())
    }
    
    /// Subscribe to market data for the given symbols on the specified exchange
    pub async fn subscribe(&self, exchange_id: ExchangeId, symbols: &[Symbol]) -> Result<()> {
        debug!("Subscribing to {} symbols on exchange: {:?}", symbols.len(), exchange_id);
        
        let connections = self.connections.read().unwrap();
        if let Some(_connection) = connections.get(&exchange_id) {
            // We need to get a mutable reference, so we'll need to rethink this
            // For now, we'll assume the connection handles subscriptions internally
            warn!("Subscription handling needs to be implemented in connection");
        } else {
            return Err(anyhow::anyhow!("No connection found for exchange: {:?}", exchange_id));
        }
        
        Ok(())
    }
    
    /// Get connection status for all exchanges
    pub fn connection_status(&self) -> Vec<(ExchangeId, bool)> {
        let status = self.status.read().unwrap();
        status.values()
            .map(|s| (s.exchange_id.clone(), s.health.is_healthy()))
            .collect()
    }
    
    /// Get detailed status for a specific exchange
    pub fn get_connection_status(&self, exchange_id: ExchangeId) -> Option<ConnectionStatus> {
        let status = self.status.read().unwrap();
        status.get(&exchange_id).cloned()
    }
    
    /// Remove a connection from the manager
    pub async fn remove_connection(&mut self, exchange_id: ExchangeId) -> Result<()> {
        info!("Removing connection for exchange: {:?}", exchange_id);
        
        // Disconnect and remove connection
        {
            let mut connections = self.connections.write().unwrap();
            if let Some(_connection) = connections.remove(&exchange_id) {
                // Note: We can't call disconnect here because we only have an Arc<dyn ExchangeConnection>
                // This needs to be redesigned
                warn!("Connection disconnect needs to be implemented");
            }
        }
        
        // Remove status
        {
            let mut status = self.status.write().unwrap();
            status.remove(&exchange_id);
        }
        
        info!("Successfully removed connection for exchange: {:?}", exchange_id);
        Ok(())
    }
    
    /// Start health monitoring for a connection
    async fn start_health_monitoring(&self, exchange_id: ExchangeId, connection: Arc<dyn ExchangeConnection>) {
        let status = self.status.clone();
        
        tokio::spawn(async move {
            let mut health_interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                health_interval.tick().await;
                
                match connection.health_check().await {
                    Ok(health) => {
                        let is_healthy = health.is_healthy();
                        
                        // Update status
                        {
                            let mut status_map = status.write().unwrap();
                            if let Some(conn_status) = status_map.get_mut(&exchange_id) {
                                let was_healthy = conn_status.health.is_healthy();
                                conn_status.health = health;
                                
                                // Log status changes
                                if was_healthy != is_healthy {
                                    if is_healthy {
                                        info!("Exchange {:?} is now healthy", exchange_id);
                                    } else {
                                        warn!("Exchange {:?} is now unhealthy", exchange_id);
                                    }
                                }
                            }
                        }
                        
                        // Attempt reconnection if unhealthy
                        if !is_healthy {
                            debug!("Attempting to reconnect to exchange: {:?}", exchange_id);
                            // Note: Reconnection logic needs to be implemented
                            warn!("Reconnection logic needs to be implemented");
                        }
                    }
                    Err(e) => {
                        error!("Health check failed for exchange {:?}: {}", exchange_id, e);
                        
                        // Update status to unhealthy
                        {
                            let mut status_map = status.write().unwrap();
                            if let Some(conn_status) = status_map.get_mut(&exchange_id) {
                                conn_status.health = ConnectionHealth::Unhealthy;
                                conn_status.error_count += 1;
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// Get statistics for all connections
    pub fn get_all_stats(&self) -> HashMap<ExchangeId, crate::protocols::base::ConnectionStats> {
        let connections = self.connections.read().unwrap();
        connections.iter()
            .map(|(id, conn)| (id.clone(), conn.get_stats()))
            .collect()
    }
}

impl Default for ExchangeConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::base::{BaseExchangeConnection, ExchangeConnection};
    use crate::types::ExchangeConfig;
    
    struct MockConnection {
        base: BaseExchangeConnection,
    }
    
    impl MockConnection {
        fn new() -> Self {
            let config = ExchangeConfig::ninjatrader("127.0.0.1:7496");
            Self {
                base: BaseExchangeConnection::new(crate::types::ExchangeId::NinjaTrader, config),
            }
        }
    }
    
    #[async_trait::async_trait]
    impl ExchangeConnection for MockConnection {
        async fn connect(&mut self) -> Result<()> {
            self.base.set_connected(true);
            Ok(())
        }
        
        async fn disconnect(&mut self) -> Result<()> {
            self.base.set_connected(false);
            Ok(())
        }
        
        fn is_connected(&self) -> bool {
            self.base.is_connected()
        }
        
        async fn health_check(&self) -> Result<ConnectionHealth> {
            Ok(ConnectionHealth::Healthy)
        }
        
        async fn subscribe_market_data(&mut self, _symbols: &[Symbol]) -> Result<()> {
            Ok(())
        }
        
        async fn unsubscribe_market_data(&mut self, _symbols: &[Symbol]) -> Result<()> {
            Ok(())
        }
        
        async fn submit_order(&mut self, _order: crate::types::OrderRequest) -> Result<crate::types::OrderId> {
            Ok("test-order-id".to_string())
        }
        
        async fn cancel_order(&mut self, _order_id: crate::types::OrderId) -> Result<()> {
            Ok(())
        }
        
        fn exchange_id(&self) -> ExchangeId {
            self.base.exchange_id()
        }
        
        fn get_stats(&self) -> crate::protocols::base::ConnectionStats {
            self.base.get_stats()
        }
    }
    
    #[tokio::test]
    async fn test_connection_manager() {
        let mut manager = ExchangeConnectionManager::new();
        
        // Add a mock connection
        let mock_conn = MockConnection::new();
        let exchange_id = crate::types::ExchangeId::NinjaTrader;
        
        manager.add_connection(exchange_id, Arc::new(mock_conn)).await.unwrap();
        
        // Check connection status
        let status = manager.connection_status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].0, exchange_id);
        assert!(status[0].1); // Should be healthy
        
        // Get detailed status
        let detailed_status = manager.get_connection_status(exchange_id);
        assert!(detailed_status.is_some());
        assert_eq!(detailed_status.unwrap().exchange_id, exchange_id);
    }
}
