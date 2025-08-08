// Test example for GzipProvider
//
// This example demonstrates how to use the new GzipProvider to read compressed 
// historical data files from the data directory.

use binance_futures::core::provider::{
    GzipProvider, GzipProviderConfig, DataProvider, ControllableProvider,
    ProviderResult, EventKind
};
use std::path::PathBuf;

fn main() -> ProviderResult<()> {
    // Initialize logging
    env_logger::init();

    println!("=== Gzip Provider Test ===");
    
    // Configure the GzipProvider
    let mut config = GzipProviderConfig::default();
    config.data_dir = PathBuf::from("data");
    config.file_pattern = "*.gz".to_string();
    config.playback_config.initial_speed = 10.0; // 10x speed for testing
    config.playback_config.auto_start = true;
    
    // Filter for specific symbol if desired
    config.symbol_filter = Some("btcfdusd".to_string());
    
    println!("Configuration:");
    println!("- Data directory: {}", config.data_dir.display());
    println!("- File pattern: {}", config.file_pattern);
    println!("- Playback speed: {}x", config.playback_config.initial_speed);
    println!("- Symbol filter: {:?}", config.symbol_filter);

    // Create the provider
    let mut provider = GzipProvider::new(config);
    
    // Initialize the provider
    println!("\nInitializing provider...");
    provider.initialize()?;
    
    println!("Provider initialized successfully!");
    println!("- Provider type: {:?}", provider.provider_type());
    println!("- Supported events: {:?}", provider.supported_events());
    println!("- Config info: {}", provider.get_config_info().unwrap_or_default());
    
    // Start the provider
    println!("\nStarting provider...");
    provider.start()?;
    
    println!("Provider started successfully!");
    println!("- Is connected: {}", provider.is_connected());
    println!("- Health check: {}", provider.health_check());
    
    // Read some events
    println!("\nReading events...");
    let mut total_events = 0;
    let mut event_counts = std::collections::HashMap::new();
    
    for i in 0..100 {  // Read 100 batches
        match provider.read_events() {
            Ok(events) => {
                if !events.is_empty() {
                    println!("Batch {}: {} events", i + 1, events.len());
                    total_events += events.len();
                    
                    // Count event types
                    for event in &events {
                        let event_name = match event {
                            binance_futures::events::EventType::BookTicker(_) => "BookTicker",
                            binance_futures::events::EventType::Trade(_) => "Trade",
                            binance_futures::events::EventType::DepthUpdate(_) => "DepthUpdate",
                            binance_futures::events::EventType::TickPrice(_) => "TickPrice",
                            _ => "Other",
                        };
                        *event_counts.entry(event_name.to_string()).or_insert(0) += 1;
                    }
                    
                    // Show first event from this batch
                    if let Some(first_event) = events.first() {
                        match first_event {
                            binance_futures::events::EventType::BookTicker(data) => {
                                println!("  Sample BookTicker: {}", 
                                    serde_json::to_string_pretty(data).unwrap_or_default().lines().take(3).collect::<Vec<_>>().join(" "));
                            },
                            binance_futures::events::EventType::Trade(data) => {
                                println!("  Sample Trade: {}", 
                                    serde_json::to_string_pretty(data).unwrap_or_default().lines().take(3).collect::<Vec<_>>().join(" "));
                            },
                            _ => println!("  Sample event: {:?}", first_event),
                        }
                    }
                } else {
                    // No events, wait a bit
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            Err(e) => {
                println!("Error reading events: {}", e);
                break;
            }
        }
        
        // Show progress every 10 batches
        if i % 10 == 9 {
            let status = provider.get_status();
            println!("Status update - Events received: {}, Healthy: {}", 
                    status.events_received, status.is_healthy);
            
            if let Some(metrics) = provider.get_performance_metrics() {
                println!("Performance - Events/sec: {:.1}, Bytes/sec: {:.1}", 
                        metrics.events_per_second, metrics.bytes_per_second);
            }
        }
    }
    
    // Test playback controls
    println!("\nTesting playback controls...");
    
    // Pause
    provider.pause()?;
    println!("Provider paused");
    
    // Try to read events (should be empty)
    let events = provider.read_events()?;
    println!("Events while paused: {}", events.len());
    
    // Resume
    provider.resume()?;
    println!("Provider resumed");
    
    // Change speed
    provider.set_playback_speed(1.0)?;
    println!("Playback speed set to 1.0x");
    
    // Get final status
    println!("\n=== Final Results ===");
    println!("Total events processed: {}", total_events);
    println!("Event type breakdown:");
    for (event_type, count) in &event_counts {
        println!("  {}: {}", event_type, count);
    }
    
    let final_status = provider.get_status();
    println!("Final status:");
    println!("  - Is connected: {}", final_status.is_connected);
    println!("  - Events received: {}", final_status.events_received);
    println!("  - Error count: {}", final_status.error_count);
    println!("  - Is healthy: {}", final_status.is_healthy);
    
    if let Some(playback_info) = provider.get_playback_info() {
        println!("Playback info:");
        println!("  - Progress: {:.2}%", playback_info.progress * 100.0);
        println!("  - Current timestamp: {}", playback_info.current_timestamp);
        println!("  - Playback speed: {}x", playback_info.playback_speed);
    }
    
    // Stop the provider
    provider.stop()?;
    println!("\nProvider stopped successfully!");
    
    Ok(())
}