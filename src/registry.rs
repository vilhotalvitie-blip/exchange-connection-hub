//! Registry for managing exchange connection factories

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info};
use crate::protocols::base::{ConnectionFactory, ExchangeFeature};
use crate::types::{ExchangeId, ExchangeConfig};

/// Registry for exchange connection factories
pub struct ExchangeConnectionRegistry {
    factories: HashMap<ExchangeId, Box<dyn ConnectionFactory>>,
}

impl ExchangeConnectionRegistry {
    /// Create a new exchange registry with default factories
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        
        // Register default factories
        registry.register_defaults();
        
        registry
    }
    
    /// Register default exchange factories
    fn register_defaults(&mut self) {
        info!("Registering NinjaTrader factory");
        self.register(Box::new(crate::protocols::ninjatrader::NinjaTraderFactory::new()));
        
        info!("Registering WebSocket factory");
        self.register(Box::new(crate::protocols::websocket::WebSocketFactory::new()));
    }
    
    /// Register a new connection factory
    pub fn register(&mut self, factory: Box<dyn ConnectionFactory>) {
        let exchange_id = factory.exchange_id();
        info!("Registering factory for exchange: {:?}", exchange_id);
        self.factories.insert(exchange_id, factory);
    }
    
    /// Create a connection for the given exchange
    pub fn create_connection(&self, exchange_id: ExchangeId, config: ExchangeConfig) -> Result<Arc<dyn crate::protocols::base::ExchangeConnection>> {
        let factory = self.factories.get(&exchange_id)
            .ok_or_else(|| anyhow::anyhow!("No factory registered for exchange: {:?}", exchange_id))?;
        
        info!("Creating connection for exchange: {:?}", exchange_id);
        Ok(Arc::from(factory.create_connection(config)?))
    }
    
    /// Get all registered exchanges
    pub fn list_exchanges(&self) -> Vec<ExchangeId> {
        self.factories.keys().cloned().collect()
    }
    
    /// Get supported features for an exchange
    pub fn get_supported_features(&self, exchange_id: ExchangeId) -> Vec<ExchangeFeature> {
        self.factories.get(&exchange_id)
            .map(|factory: &Box<dyn ConnectionFactory>| factory.supported_features())
            .unwrap_or_default()
    }
    
    /// Check if an exchange is supported
    pub fn is_supported(&self, exchange_id: ExchangeId) -> bool {
        self.factories.contains_key(&exchange_id)
    }
    
    /// Get information about all registered exchanges
    pub fn get_exchange_info(&self) -> Vec<RegisteredExchangeInfo> {
        self.factories.values()
            .map(|factory: &Box<dyn ConnectionFactory>| RegisteredExchangeInfo {
                exchange_id: factory.exchange_id(),
                supported_features: factory.supported_features(),
            })
            .collect()
    }
}

/// Information about a registered exchange
#[derive(Debug, Clone)]
pub struct RegisteredExchangeInfo {
    pub exchange_id: ExchangeId,
    pub supported_features: Vec<ExchangeFeature>,
}

impl Default for ExchangeConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::base::{ConnectionFactory};
    use crate::types::{ExchangeConfig, ExchangeId};
    
    struct MockFactory;
    
    impl ConnectionFactory for MockFactory {
        fn create_connection(&self, _config: ExchangeConfig) -> Result<Arc<dyn crate::protocols::base::ExchangeConnection>> {
            Err(anyhow::anyhow!("Mock factory"))
        }
        
        fn exchange_id(&self) -> ExchangeId {
            ExchangeId::NinjaTrader
        }
        
        fn supported_features(&self) -> Vec<ExchangeFeature> {
            vec![ExchangeFeature::MarketData, ExchangeFeature::OrderExecution]
        }
    }
    
    #[test]
    fn test_registry() {
        let mut registry = ExchangeRegistry::new();
        
        // Register mock factory
        registry.register(Box::new(MockFactory));
        
        // Check if exchange is supported
        assert!(registry.is_supported(ExchangeId::NinjaTrader));
        assert!(!registry.is_supported(ExchangeId::InteractiveBrokers));
        
        // List exchanges
        let exchanges = registry.list_exchanges();
        assert!(exchanges.contains(&ExchangeId::NinjaTrader));
        
        // Get supported features
        let features = registry.get_supported_features(ExchangeId::NinjaTrader);
        assert!(features.contains(&ExchangeFeature::MarketData));
        assert!(features.contains(&ExchangeFeature::OrderExecution));
        
        // Get exchange info
        let info = registry.get_exchange_info();
        assert!(!info.is_empty());
    }
}
