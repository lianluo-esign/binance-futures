// Startup Configuration Validator
// Validates configuration against provider implementations at startup
// If validation fails, the program exits with a clear error message

use super::{ConfigError, GlobalConfig, ProviderIdentity};
use crate::core::provider::{BinanceProvider, GzipProvider};
use std::process;

/// Startup validation error that causes program termination
#[derive(Debug)]
pub struct StartupValidationError {
    pub provider_name: String,
    pub error_type: String,
    pub expected: String,
    pub found: String,
    pub canonical_name: String,
}

impl std::fmt::Display for StartupValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "🚨 CONFIGURATION ERROR - STARTUP ABORTED")?;
        writeln!(f, "Provider: {}", self.provider_name)?;
        writeln!(f, "Error: {}", self.error_type)?;
        writeln!(f, "Expected: '{}'", self.expected)?;
        writeln!(f, "Found: '{}'", self.found)?;
        writeln!(f, "Canonical Name: '{}'", self.canonical_name)?;
        writeln!(f)?;
        writeln!(f, "📋 Fix Required:")?;
        writeln!(f, "Update your configuration files to use the correct provider name.")?;
        writeln!(f, "The provider implementation defines the canonical name that MUST be used.")?;
        Ok(())
    }
}

/// Validate all configurations against provider implementations
/// Exits the program with error code 1 if validation fails
pub fn validate_configuration_or_exit(config: &GlobalConfig) {
    println!("🔍 Validating configuration against provider implementations...");
    
    let mut errors = Vec::new();
    
    for provider_meta in &config.providers.configs {
        match validate_provider_against_implementation(&provider_meta.name, &provider_meta.provider_type) {
            Ok(_) => {
                println!("   ✅ {} validated successfully", provider_meta.name);
            }
            Err(error) => {
                errors.push(error);
            }
        }
    }
    
    if !errors.is_empty() {
        eprintln!("\n❌ Configuration validation failed!");
        eprintln!("Found {} error(s):\n", errors.len());
        
        for (i, error) in errors.iter().enumerate() {
            eprintln!("Error {}: {}", i + 1, error);
        }
        
        eprintln!("🛑 Program terminated due to configuration errors.");
        eprintln!("Fix the configuration and try again.");
        process::exit(1);
    }
    
    println!("   ✅ All provider configurations validated successfully!");
}

/// Validate a single provider configuration against its implementation
fn validate_provider_against_implementation(
    config_name: &str,
    config_type: &str,
) -> Result<(), StartupValidationError> {
    match config_name {
        // Validate Binance provider
        name if name.contains("binance") => {
            let canonical_name = BinanceProvider::CANONICAL_NAME;
            let canonical_type = BinanceProvider::CANONICAL_TYPE;
            
            if config_name != canonical_name {
                return Err(StartupValidationError {
                    provider_name: config_name.to_string(),
                    error_type: "Provider name mismatch".to_string(),
                    expected: canonical_name.to_string(),
                    found: config_name.to_string(),
                    canonical_name: canonical_name.to_string(),
                });
            }
            
            if config_type != canonical_type {
                return Err(StartupValidationError {
                    provider_name: config_name.to_string(),
                    error_type: "Provider type mismatch".to_string(),
                    expected: canonical_type.to_string(),
                    found: config_type.to_string(),
                    canonical_name: canonical_name.to_string(),
                });
            }
        }
        
        // Validate Gzip provider
        name if name.contains("gzip") || name.contains("historical") => {
            let canonical_name = GzipProvider::CANONICAL_NAME;
            let canonical_type = GzipProvider::CANONICAL_TYPE;
            
            if config_name != canonical_name {
                return Err(StartupValidationError {
                    provider_name: config_name.to_string(),
                    error_type: "Provider name mismatch".to_string(),
                    expected: canonical_name.to_string(),
                    found: config_name.to_string(),
                    canonical_name: canonical_name.to_string(),
                });
            }
            
            if config_type != canonical_type {
                return Err(StartupValidationError {
                    provider_name: config_name.to_string(),
                    error_type: "Provider type mismatch".to_string(),
                    expected: canonical_type.to_string(),
                    found: config_type.to_string(),
                    canonical_name: canonical_name.to_string(),
                });
            }
        }
        
        // Unknown provider
        _ => {
            return Err(StartupValidationError {
                provider_name: config_name.to_string(),
                error_type: "Unknown provider".to_string(),
                expected: "A registered provider name".to_string(),
                found: config_name.to_string(),
                canonical_name: "N/A".to_string(),
            });
        }
    }
    
    Ok(())
}

/// Get the canonical name for a provider type (helper function)
pub fn get_canonical_name_for_type(provider_type: &str) -> Option<&'static str> {
    match provider_type {
        "BinanceWebSocket" => Some(BinanceProvider::CANONICAL_NAME),
        "GzipProvider" => Some(GzipProvider::CANONICAL_NAME),
        _ => None,
    }
}

/// Get all canonical provider names (for documentation/help)
pub fn get_all_canonical_names() -> Vec<(&'static str, &'static str)> {
    vec![
        (BinanceProvider::CANONICAL_NAME, BinanceProvider::CANONICAL_TYPE),
        (GzipProvider::CANONICAL_NAME, GzipProvider::CANONICAL_TYPE),
    ]
}

/// Print all canonical provider names (for user reference)
pub fn print_canonical_provider_names() {
    println!("📋 Valid Provider Names (defined by implementations):");
    println!("{}", "=".repeat(60));
    
    let names = get_all_canonical_names();
    for (name, provider_type) in names {
        println!("   • Name: '{}' → Type: '{}'", name, provider_type);
    }
    
    println!("\n⚠️  These names are FIXED by the provider implementations.");
    println!("   Update your configuration files to use these exact names.");
}