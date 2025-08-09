// Provider Name to Implementation File Mapping
// This file explicitly declares the mapping between configuration provider names
// and their corresponding implementation files to eliminate guesswork

use std::collections::HashMap;
use lazy_static::lazy_static;

/// Provider mapping structure that defines the relationship between
/// configuration provider names and their implementation details
#[derive(Debug, Clone)]
pub struct ProviderMapping {
    /// Configuration name (used in config.toml)
    pub config_name: &'static str,
    /// Provider type identifier (used for type matching)
    pub provider_type: &'static str,
    /// Implementation file path (relative to src/)
    pub implementation_file: &'static str,
    /// Main implementation struct name
    pub struct_name: &'static str,
    /// Configuration struct name
    pub config_struct: &'static str,
}

/// Global provider mapping registry
/// This is the SINGLE SOURCE OF TRUTH for provider mappings
lazy_static! {
    pub static ref PROVIDER_MAPPINGS: HashMap<&'static str, ProviderMapping> = {
        let mut map = HashMap::new();
        
        // Binance WebSocket Provider Mapping
        map.insert("binance_websocket", ProviderMapping {
            config_name: "binance_websocket",
            provider_type: "BinanceWebSocket", 
            implementation_file: "core/provider/binance_provider.rs",
            struct_name: "BinanceProvider",
            config_struct: "BinanceWebSocketConfig",
        });
        
        // Gzip Historical Data Provider Mapping
        map.insert("gzip_historical", ProviderMapping {
            config_name: "gzip_historical",
            provider_type: "GzipProvider",
            implementation_file: "core/provider/gzip_provider.rs", 
            struct_name: "GzipProvider",
            config_struct: "GzipProviderConfig",
        });
        
        // Binance REST API Provider Mapping
        map.insert("binance_rest", ProviderMapping {
            config_name: "binance_rest",
            provider_type: "BinanceRest",
            implementation_file: "core/provider/binance_provider.rs", // Same file, different functionality
            struct_name: "BinanceRestProvider", // TODO: Implement this struct
            config_struct: "BinanceRestConfig",
        });
        
        // Historical File Provider Mapping
        map.insert("historical_file", ProviderMapping {
            config_name: "historical_file",
            provider_type: "HistoricalFile",
            implementation_file: "core/provider/gzip_historical_provider.rs",
            struct_name: "HistoricalDataProvider",
            config_struct: "HistoricalDataConfig",
        });
        
        // Mock Provider Mapping
        map.insert("mock_provider", ProviderMapping {
            config_name: "mock_provider",
            provider_type: "Mock",
            implementation_file: "core/provider/mock_provider.rs", // TODO: Create this file
            struct_name: "MockProvider", // TODO: Implement this struct
            config_struct: "MockProviderConfig",
        });
        
        map
    };
}

/// Get provider mapping by configuration name
pub fn get_provider_mapping(config_name: &str) -> Option<&ProviderMapping> {
    PROVIDER_MAPPINGS.get(config_name)
}

/// Get all registered provider names
pub fn get_all_provider_names() -> Vec<&'static str> {
    PROVIDER_MAPPINGS.keys().cloned().collect()
}

/// Validate that a provider name exists in the mapping
pub fn is_valid_provider_name(name: &str) -> bool {
    PROVIDER_MAPPINGS.contains_key(name)
}

/// Validate configuration against mappings
pub fn validate_provider_config(config_name: &str, provider_type: &str) -> Result<(), String> {
    match get_provider_mapping(config_name) {
        Some(mapping) => {
            if mapping.provider_type == provider_type {
                Ok(())
            } else {
                Err(format!(
                    "Type mismatch for provider '{}': expected '{}', got '{}'",
                    config_name, mapping.provider_type, provider_type
                ))
            }
        }
        None => Err(format!("Unknown provider name: '{}'", config_name))
    }
}

/// Print all provider mappings for debugging
pub fn print_all_mappings() {
    println!("Provider Name to Implementation Mappings:");
    println!("{}", "=".repeat(60));
    
    for (_name, mapping) in PROVIDER_MAPPINGS.iter() {
        println!("Config Name: {}", mapping.config_name);
        println!("Provider Type: {}", mapping.provider_type);
        println!("Implementation: src/{}", mapping.implementation_file);
        println!("Struct Name: {}", mapping.struct_name);
        println!("Config Struct: {}", mapping.config_struct);
        println!("{}", "-".repeat(40));
    }
}

/// Generate markdown documentation for mappings
pub fn generate_mapping_docs() -> String {
    let mut docs = String::new();
    docs.push_str("# Provider Implementation Mappings\n\n");
    docs.push_str("This document shows the explicit mapping between configuration provider names and their implementations.\n\n");
    docs.push_str("| Config Name | Provider Type | Implementation File | Struct Name | Config Struct |\n");
    docs.push_str("|-------------|---------------|---------------------|-------------|---------------|\n");
    
    for (_, mapping) in PROVIDER_MAPPINGS.iter() {
        docs.push_str(&format!(
            "| {} | {} | `src/{}` | `{}` | `{}` |\n",
            mapping.config_name,
            mapping.provider_type, 
            mapping.implementation_file,
            mapping.struct_name,
            mapping.config_struct
        ));
    }
    
    docs.push_str("\n## Usage in config.toml\n\n");
    docs.push_str("```toml\n");
    docs.push_str("[[providers.config]]\n");
    docs.push_str("name = \"binance_websocket\"     # Must match Config Name in table above\n");
    docs.push_str("type = \"BinanceWebSocket\"       # Must match Provider Type in table above\n");
    docs.push_str("enabled = true\n");
    docs.push_str("config_file = \"configs/providers/binance_websocket.toml\"\n");
    docs.push_str("```\n");
    
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_binance_websocket_mapping() {
        let mapping = get_provider_mapping("binance_websocket").unwrap();
        assert_eq!(mapping.config_name, "binance_websocket");
        assert_eq!(mapping.provider_type, "BinanceWebSocket");
        assert_eq!(mapping.implementation_file, "core/provider/binance_provider.rs");
        assert_eq!(mapping.struct_name, "BinanceProvider");
    }
    
    #[test]
    fn test_gzip_mapping() {
        let mapping = get_provider_mapping("gzip_historical").unwrap();
        assert_eq!(mapping.config_name, "gzip_historical");
        assert_eq!(mapping.provider_type, "GzipProvider");
        assert_eq!(mapping.implementation_file, "core/provider/gzip_provider.rs");
        assert_eq!(mapping.struct_name, "GzipProvider");
    }
    
    #[test]
    fn test_validation() {
        assert!(validate_provider_config("binance_websocket", "BinanceWebSocket").is_ok());
        assert!(validate_provider_config("binance_websocket", "WrongType").is_err());
        assert!(validate_provider_config("unknown_provider", "AnyType").is_err());
    }
    
    #[test]
    fn test_provider_names() {
        let names = get_all_provider_names();
        assert!(names.contains(&"binance_websocket"));
        assert!(names.contains(&"gzip_historical"));
    }
}