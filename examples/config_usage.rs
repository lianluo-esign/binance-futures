// Example demonstrating the configuration system usage
// Shows how to load configurations and create providers

use binance_futures::config::{
    ConfigManager, init_config, get_config, get_config_mut
};
use binance_futures::core::provider::{
    ConfiguredProviderFactory, DataProvider, ProviderConfigAdapter
};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Configuration System Demo ===\n");

    // Method 1: Using global configuration singleton
    demo_global_config()?;
    
    println!("\n" + "=".repeat(50).as_str() + "\n");
    
    // Method 2: Using explicit configuration manager
    demo_explicit_config()?;
    
    println!("\n" + "=".repeat(50).as_str() + "\n");
    
    // Method 3: Using configuration factory to create providers
    demo_provider_factory()?;
    
    Ok(())
}

/// Demonstrate using global configuration singleton
fn demo_global_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("Method 1: Global Configuration Singleton");
    println!("-" * 40);
    
    // Initialize global configuration
    match init_config() {
        Ok(_) => println!("✓ Configuration loaded successfully"),
        Err(e) => {
            println!("⚠ Using default configuration: {}", e);
            // Load with defaults
            get_config_mut().load_or_default();
        }
    }
    
    // Access global configuration
    {
        let config = get_config();
        
        println!("\nSystem Configuration:");
        println!("  Name: {}", config.global().system.name);
        println!("  Version: {}", config.global().system.version);
        println!("  Log Level: {}", config.global().system.log_level);
        
        println!("\nRuntime Configuration:");
        println!("  CPU Affinity: {}", config.global().runtime.cpu_affinity);
        println!("  CPU Cores: {:?}", config.global().runtime.cpu_cores);
        println!("  Thread Pool Size: {}", config.global().runtime.thread_pool_size);
        
        println!("\nActive Providers:");
        for provider_name in &config.global().providers.active {
            println!("  - {}", provider_name);
        }
    }
    
    // Modify configuration
    {
        let mut config = get_config_mut();
        config.global_mut().system.log_level = "debug".to_string();
        println!("\n✓ Modified log level to: debug");
    }
    
    Ok(())
}

/// Demonstrate using explicit configuration manager
fn demo_explicit_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("Method 2: Explicit Configuration Manager");
    println!("-" * 40);
    
    // Create configuration manager with custom path
    let mut config_manager = ConfigManager::with_path("config.toml");
    
    // Load or use defaults
    config_manager.load_or_default();
    
    println!("✓ Configuration manager created");
    
    // Access provider configurations
    println!("\nProvider Configurations:");
    
    // Check Binance WebSocket configuration
    if let Some(provider_config) = config_manager.provider("binance_websocket") {
        match provider_config {
            binance_futures::config::ProviderConfig::BinanceWebSocket(ws_config) => {
                println!("\nBinance WebSocket Provider:");
                println!("  URL: {}", ws_config.connection.base_url);
                println!("  Symbols: {:?}", ws_config.subscription.symbols);
                println!("  Streams: {:?}", ws_config.subscription.streams);
                println!("  Reconnect: {}", ws_config.connection.reconnect_enabled);
                println!("  Buffer Size: {}", ws_config.performance.buffer_size);
            }
            _ => {}
        }
    }
    
    // Check Gzip provider configuration
    if let Some(provider_config) = config_manager.provider("gzip_historical") {
        match provider_config {
            binance_futures::config::ProviderConfig::Gzip(gzip_config) => {
                println!("\nGzip Historical Provider:");
                println!("  Data Directory: {}", gzip_config.data_source.data_directory);
                println!("  File Pattern: {}", gzip_config.data_source.file_pattern);
                println!("  Symbols: {:?}", gzip_config.data_source.symbols);
                println!("  Playback Speed: {}x", gzip_config.playback.initial_speed);
                println!("  Auto Start: {}", gzip_config.playback.auto_start);
            }
            _ => {}
        }
    }
    
    // Get active providers
    println!("\nActive Provider Details:");
    for provider in config_manager.active_providers() {
        match provider {
            binance_futures::config::ProviderConfig::BinanceWebSocket(config) => {
                println!("  {} (WebSocket): {} symbols configured", 
                    config.provider.name, 
                    config.subscription.symbols.len()
                );
            }
            binance_futures::config::ProviderConfig::Gzip(config) => {
                println!("  {} (Historical): {} directory", 
                    config.provider.name,
                    config.data_source.data_directory
                );
            }
            _ => {}
        }
    }
    
    Ok(())
}

/// Demonstrate using configuration factory to create providers
fn demo_provider_factory() -> Result<(), Box<dyn std::error::Error>> {
    println!("Method 3: Provider Factory with Configuration");
    println!("-" * 40);
    
    // Create configuration manager
    let mut config_manager = ConfigManager::new();
    config_manager.load_or_default();
    
    // Create provider factory
    let config_arc = Arc::new(config_manager);
    let factory = ConfiguredProviderFactory::new(config_arc.clone());
    
    println!("✓ Provider factory created");
    
    // Create specific provider by name
    println!("\nCreating providers from configuration:");
    
    // Try to create Binance WebSocket provider
    match factory.create_provider("binance_websocket") {
        Ok(_provider) => {
            println!("  ✓ Created Binance WebSocket provider");
            // Provider is ready to use
            // provider.connect().await?;
            // provider.subscribe("BTCFDUSD").await?;
        }
        Err(e) => {
            println!("  ✗ Failed to create Binance WebSocket provider: {}", e);
        }
    }
    
    // Try to create Gzip provider
    match factory.create_provider("gzip_historical") {
        Ok(_provider) => {
            println!("  ✓ Created Gzip Historical provider");
            // Provider is ready to use
            // provider.start().await?;
        }
        Err(e) => {
            println!("  ✗ Failed to create Gzip provider: {}", e);
        }
    }
    
    // Create all active providers at once
    println!("\nCreating all active providers:");
    let providers = factory.create_active_providers();
    println!("  ✓ Created {} active providers", providers.len());
    
    // Demonstrate provider usage
    for (i, provider) in providers.iter().enumerate() {
        println!("\nProvider {}: {}", i + 1, provider.name());
        println!("  Status: {:?}", provider.status());
        println!("  Provider Type: {}", provider.provider_type());
        
        // Each provider is ready to use
        // Example: provider.start().await?;
    }
    
    Ok(())
}

// Helper function to demonstrate runtime configuration updates
#[allow(dead_code)]
fn update_provider_config_example() -> Result<(), Box<dyn std::error::Error>> {
    use binance_futures::config::BinanceWebSocketConfig;
    use binance_futures::core::provider::BinanceProvider;
    
    // Load configuration
    let ws_config = BinanceWebSocketConfig::default();
    
    // Create provider from configuration
    let mut provider = BinanceProvider::from_config(&ws_config)?;
    
    // Update configuration at runtime
    let mut new_config = ws_config.clone();
    new_config.subscription.symbols = vec!["ETHFDUSD".to_string(), "BNBFDUSD".to_string()];
    new_config.performance.buffer_size = 20000;
    
    // Apply new configuration
    provider.update_from_config(&new_config)?;
    
    println!("Provider configuration updated successfully");
    
    Ok(())
}