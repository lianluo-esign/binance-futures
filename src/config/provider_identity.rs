// Provider Identity Trait
// Each provider implementation defines its canonical name and type
// This eliminates the need for external mapping tables

/// Trait that all providers must implement to declare their canonical identity
pub trait ProviderIdentity {
    /// The canonical name that must be used in configuration files
    /// This is the source of truth for the provider's identity
    const CANONICAL_NAME: &'static str;
    
    /// The canonical type identifier for this provider
    const CANONICAL_TYPE: &'static str;
    
    /// Human-readable display name for this provider
    const DISPLAY_NAME: &'static str;
    
    /// Version of the provider implementation
    const VERSION: &'static str;
    
    /// Get the canonical name (same as constant, but accessible via trait object)
    fn canonical_name(&self) -> &'static str {
        Self::CANONICAL_NAME
    }
    
    /// Get the canonical type (same as constant, but accessible via trait object)
    fn canonical_type(&self) -> &'static str {
        Self::CANONICAL_TYPE
    }
    
    /// Get the display name
    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }
    
    /// Get the version
    fn version(&self) -> &'static str {
        Self::VERSION
    }
    
    /// Validate that a configuration name matches this provider's canonical name
    fn validate_config_name(&self, config_name: &str) -> Result<(), String> {
        if config_name == Self::CANONICAL_NAME {
            Ok(())
        } else {
            Err(format!(
                "Configuration name mismatch for {}: expected '{}', got '{}'",
                Self::DISPLAY_NAME,
                Self::CANONICAL_NAME,
                config_name
            ))
        }
    }
    
    /// Validate that a configuration type matches this provider's canonical type
    fn validate_config_type(&self, config_type: &str) -> Result<(), String> {
        if config_type == Self::CANONICAL_TYPE {
            Ok(())
        } else {
            Err(format!(
                "Configuration type mismatch for {}: expected '{}', got '{}'",
                Self::DISPLAY_NAME,
                Self::CANONICAL_TYPE,
                config_type
            ))
        }
    }
}