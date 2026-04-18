//! Market provider type definitions.
//!
//! Lookup functions (find_market_provider, get_market_provider_by_id, etc.)
//! live in nxr-sdk::providers.

use crate::constants::DataEntry;

/// Market provider with normalized search key
#[derive(Debug, Clone, PartialEq)]
pub struct MarketProvider {
    pub id: u16,
    pub name: String,
}

/// Market provider search result
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderMatch {
    pub provider: MarketProvider,
    pub confidence: f64,
}

impl From<&DataEntry> for MarketProvider {
    fn from(entry: &DataEntry) -> Self {
        Self { id: entry.id as u16, name: entry.name.to_string() }
    }
}
