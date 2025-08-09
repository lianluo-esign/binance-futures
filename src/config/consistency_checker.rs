// Configuration Consistency Checker
// Ensures naming consistency between global config and provider configs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::config::{GlobalConfig, get_provider_mapping, validate_provider_config};

#[derive(Debug, Serialize, Deserialize)]
struct ProviderConfigHeader {
    provider: ProviderInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProviderInfo {
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    version: String,
}

#[derive(Debug, Clone)]
pub struct ConsistencyError {
    pub provider_name: String,
    pub error_type: String,
    pub expected: String,
    pub found: String,
    pub file_path: String,
}

impl std::fmt::Display for ConsistencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {} - Expected: '{}', Found: '{}' in {}",
               self.provider_name, self.error_type, self.expected, self.found, self.file_path)
    }
}

pub struct ConfigurationConsistencyChecker {
    global_config: GlobalConfig,
}

impl ConfigurationConsistencyChecker {
    /// Create new consistency checker
    pub fn new(global_config: GlobalConfig) -> Self {
        Self { global_config }
    }

    /// Check all configuration consistency
    pub fn check_all_consistency(&self) -> Result<(), Vec<ConsistencyError>> {
        let mut errors = Vec::new();

        for provider_meta in &self.global_config.providers.configs {
            if let Err(mut provider_errors) = self.check_provider_consistency(provider_meta) {
                errors.append(&mut provider_errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check single provider consistency
    fn check_provider_consistency(
        &self,
        provider_meta: &crate::config::ProviderMetadata,
    ) -> Result<(), Vec<ConsistencyError>> {
        let mut errors = Vec::new();

        // Check if provider config file exists
        let config_file_path = &provider_meta.config_file;
        if !Path::new(config_file_path).exists() {
            errors.push(ConsistencyError {
                provider_name: provider_meta.name.clone(),
                error_type: "Missing Config File".to_string(),
                expected: "File should exist".to_string(),
                found: "File not found".to_string(),
                file_path: config_file_path.clone(),
            });
            return Err(errors);
        }

        // Read and parse provider config header
        match fs::read_to_string(config_file_path) {
            Ok(content) => {
                match toml::from_str::<ProviderConfigHeader>(&content) {
                    Ok(provider_config) => {
                        // Check name consistency
                        if provider_config.provider.name != provider_meta.name {
                            errors.push(ConsistencyError {
                                provider_name: provider_meta.name.clone(),
                                error_type: "Name Mismatch".to_string(),
                                expected: provider_meta.name.clone(),
                                found: provider_config.provider.name.clone(),
                                file_path: config_file_path.clone(),
                            });
                        }

                        // Check type consistency
                        if provider_config.provider.provider_type != provider_meta.provider_type {
                            errors.push(ConsistencyError {
                                provider_name: provider_meta.name.clone(),
                                error_type: "Type Mismatch".to_string(),
                                expected: provider_meta.provider_type.clone(),
                                found: provider_config.provider.provider_type.clone(),
                                file_path: config_file_path.clone(),
                            });
                        }

                        // Check against mapping registry
                        if let Err(mapping_error) = validate_provider_config(
                            &provider_meta.name,
                            &provider_meta.provider_type,
                        ) {
                            errors.push(ConsistencyError {
                                provider_name: provider_meta.name.clone(),
                                error_type: "Mapping Validation".to_string(),
                                expected: "Valid mapping".to_string(),
                                found: mapping_error,
                                file_path: config_file_path.clone(),
                            });
                        }
                    }
                    Err(parse_error) => {
                        errors.push(ConsistencyError {
                            provider_name: provider_meta.name.clone(),
                            error_type: "TOML Parse Error".to_string(),
                            expected: "Valid TOML".to_string(),
                            found: parse_error.to_string(),
                            file_path: config_file_path.clone(),
                        });
                    }
                }
            }
            Err(io_error) => {
                errors.push(ConsistencyError {
                    provider_name: provider_meta.name.clone(),
                    error_type: "File Read Error".to_string(),
                    expected: "Readable file".to_string(),
                    found: io_error.to_string(),
                    file_path: config_file_path.clone(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Generate consistency report
    pub fn generate_consistency_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Configuration Consistency Report\n\n");

        match self.check_all_consistency() {
            Ok(_) => {
                report.push_str("✅ **All configurations are consistent!**\n\n");
                
                // Show successful validations
                report.push_str("## Validated Providers\n\n");
                for provider_meta in &self.global_config.providers.configs {
                    if let Some(mapping) = get_provider_mapping(&provider_meta.name) {
                        report.push_str(&format!(
                            "- ✅ `{}` → `{}` ({})\n",
                            provider_meta.name,
                            mapping.struct_name,
                            mapping.implementation_file
                        ));
                    }
                }
            }
            Err(errors) => {
                report.push_str(&format!("❌ **Found {} consistency errors:**\n\n", errors.len()));
                
                for (i, error) in errors.iter().enumerate() {
                    report.push_str(&format!("{}. **{}**\n", i + 1, error.provider_name));
                    report.push_str(&format!("   - Error: {}\n", error.error_type));
                    report.push_str(&format!("   - Expected: `{}`\n", error.expected));
                    report.push_str(&format!("   - Found: `{}`\n", error.found));
                    report.push_str(&format!("   - File: `{}`\n\n", error.file_path));
                }

                report.push_str("## How to Fix\n\n");
                report.push_str("1. Ensure provider `name` in individual config files matches `name` in config.toml\n");
                report.push_str("2. Ensure provider `type` in individual config files matches `type` in config.toml\n");
                report.push_str("3. Check that all provider names are registered in `provider_mapping.rs`\n");
            }
        }

        report
    }

    /// Auto-fix simple consistency issues
    pub fn auto_fix_consistency(&self) -> Result<Vec<String>, String> {
        let mut fixed_files = Vec::new();

        for provider_meta in &self.global_config.providers.configs {
            match self.fix_provider_config(provider_meta) {
                Ok(Some(file_path)) => fixed_files.push(file_path),
                Ok(None) => {}, // No fix needed
                Err(e) => return Err(format!("Failed to fix {}: {}", provider_meta.name, e)),
            }
        }

        Ok(fixed_files)
    }

    fn fix_provider_config(
        &self,
        provider_meta: &crate::config::ProviderMetadata,
    ) -> Result<Option<String>, String> {
        let config_file_path = &provider_meta.config_file;
        
        if !Path::new(config_file_path).exists() {
            return Err("Config file does not exist".to_string());
        }

        let content = fs::read_to_string(config_file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Simple regex-based fix for common issues
        let mut modified = false;
        let mut new_content = content;

        // Fix name field
        let name_pattern = format!(r#"name = "[^"]*""#);
        let name_replacement = format!(r#"name = "{}""#, provider_meta.name);
        if !new_content.contains(&name_replacement) {
            new_content = regex::Regex::new(&name_pattern)
                .unwrap()
                .replace(&new_content, name_replacement.as_str())
                .to_string();
            modified = true;
        }

        // Fix type field
        let type_pattern = format!(r#"type = "[^"]*""#);
        let type_replacement = format!(r#"type = "{}""#, provider_meta.provider_type);
        if !new_content.contains(&type_replacement) {
            new_content = regex::Regex::new(&type_pattern)
                .unwrap()
                .replace(&new_content, type_replacement.as_str())
                .to_string();
            modified = true;
        }

        if modified {
            fs::write(config_file_path, new_content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(Some(config_file_path.clone()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GlobalConfig;

    #[test]
    fn test_consistency_checker() {
        let config = GlobalConfig::default();
        let checker = ConfigurationConsistencyChecker::new(config);
        
        // This would normally check actual files
        // In tests, we'd mock the filesystem
        assert!(true); // Placeholder test
    }
}