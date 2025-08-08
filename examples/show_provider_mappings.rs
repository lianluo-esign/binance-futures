// Example to show explicit provider mappings
// This demonstrates the clear relationship between config names and implementation files

use binance_futures::config::{
    print_all_mappings, 
    generate_mapping_docs, 
    get_provider_mapping,
    validate_provider_config,
    get_all_provider_names
};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Provider Name to Implementation Mapping");
    println!("{}", "=".repeat(60));
    
    // Show all explicit mappings
    print_all_mappings();
    
    // Show specific mapping lookup
    println!("\n📋 Specific Mapping Lookups:");
    println!("{}", "-".repeat(40));
    
    if let Some(mapping) = get_provider_mapping("binance_websocket") {
        println!("✅ binance_websocket:");
        println!("   Implementation: src/{}", mapping.implementation_file);
        println!("   Struct Name: {}", mapping.struct_name);
        println!("   Config Struct: {}", mapping.config_struct);
    }
    
    if let Some(mapping) = get_provider_mapping("gzip_historical") {
        println!("✅ gzip_historical:");
        println!("   Implementation: src/{}", mapping.implementation_file);
        println!("   Struct Name: {}", mapping.struct_name);
        println!("   Config Struct: {}", mapping.config_struct);
    }
    
    // Show validation examples
    println!("\n🔍 Configuration Validation:");
    println!("{}", "-".repeat(40));
    
    // Valid configurations
    match validate_provider_config("binance_websocket", "BinanceWebSocket") {
        Ok(_) => println!("✅ binance_websocket + BinanceWebSocket: Valid"),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    match validate_provider_config("gzip_historical", "GzipProvider") {
        Ok(_) => println!("✅ gzip_historical + GzipProvider: Valid"),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Invalid configurations
    match validate_provider_config("binance_websocket", "WrongType") {
        Ok(_) => println!("✅ Valid"),
        Err(e) => println!("❌ binance_websocket + WrongType: {}", e),
    }
    
    match validate_provider_config("unknown_provider", "AnyType") {
        Ok(_) => println!("✅ Valid"),
        Err(e) => println!("❌ unknown_provider: {}", e),
    }
    
    // Show all valid provider names
    println!("\n📝 All Valid Provider Names:");
    println!("{}", "-".repeat(40));
    let names = get_all_provider_names();
    for name in &names {
        println!("   • {}", name);
    }
    
    // Generate and save documentation
    println!("\n📚 Generating Documentation:");
    println!("{}", "-".repeat(40));
    let docs = generate_mapping_docs();
    
    match fs::write("PROVIDER_MAPPINGS.md", &docs) {
        Ok(_) => {
            println!("✅ Documentation generated: PROVIDER_MAPPINGS.md");
            println!("   Preview:");
            for line in docs.lines().take(10) {
                println!("   {}", line);
            }
            if docs.lines().count() > 10 {
                println!("   ... ({} more lines)", docs.lines().count() - 10);
            }
        }
        Err(e) => println!("❌ Failed to write documentation: {}", e),
    }
    
    println!("\n🎯 Summary:");
    println!("   • Total registered providers: {}", names.len());
    println!("   • All mappings are explicitly declared in src/config/provider_mapping.rs");
    println!("   • No guesswork - every config name has a clear implementation file");
    println!("   • Configuration validation prevents mismatches");
    
    Ok(())
}