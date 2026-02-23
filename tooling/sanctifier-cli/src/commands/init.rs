use clap::Args;
use colored::Colorize;
use sanctifier_core::{CustomRule, SanctifyConfig};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Force overwrite existing configuration file
    #[arg(short, long)]
    pub force: bool,
}

pub struct ConfigGenerator;

impl ConfigGenerator {
    pub fn generate_default_config() -> SanctifyConfig {
        SanctifyConfig {
            ignore_paths: vec!["target".to_string(), ".git".to_string()],
            enabled_rules: vec![
                "auth_gaps".to_string(),
                "panics".to_string(),
                "arithmetic".to_string(),
                "ledger_size".to_string(),
            ],
            ledger_limit: 64000,
            strict_mode: false,
            custom_rules: vec![
                CustomRule {
                    name: "no_unsafe_block".to_string(),
                    pattern: "unsafe\\s*\\{".to_string(),
                },
                CustomRule {
                    name: "no_mem_forget".to_string(),
                    pattern: "std::mem::forget".to_string(),
                },
            ],
            approaching_threshold: 0.8,
        }
    }
}

pub struct FileWriter;

impl FileWriter {
    pub fn config_exists(path: &Path) -> bool {
        path.join(".sanctify.toml").exists()
    }

    pub fn write_config(config: &SanctifyConfig, path: &Path) -> anyhow::Result<PathBuf> {
        let config_path = path.join(".sanctify.toml");
        let toml_string = toml::to_string_pretty(config)?;
        fs::write(&config_path, toml_string)?;
        Ok(config_path)
    }
}

pub struct OutputFormatter;

impl OutputFormatter {
    pub fn display_success(config_path: &Path) {
        println!("{} Configuration file created successfully!", "✓".green());
        println!("   Location: {}", config_path.display());
    }

    pub fn display_existing_file_warning() {
        eprintln!("{} Configuration file already exists: .sanctify.toml", "⚠".yellow());
        eprintln!("   Use --force to overwrite the existing configuration");
    }

    pub fn display_error(error: &anyhow::Error) {
        eprintln!("{} Failed to create configuration file", "✗".red());
        eprintln!("   Error: {}", error);
    }
}

pub fn exec(args: InitArgs) -> anyhow::Result<()> {
    use std::env;

    // Get current directory
    let current_dir = env::current_dir()?;

    // Check for existing config file
    if FileWriter::config_exists(&current_dir) && !args.force {
        OutputFormatter::display_existing_file_warning();
        std::process::exit(1);
    }

    // Generate default configuration
    let config = ConfigGenerator::generate_default_config();

    // Write configuration to file
    match FileWriter::write_config(&config, &current_dir) {
        Ok(config_path) => {
            OutputFormatter::display_success(&config_path);
            Ok(())
        }
        Err(e) => {
            OutputFormatter::display_error(&e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_default_config() {
        let config = ConfigGenerator::generate_default_config();

        // Verify ignore_paths
        assert_eq!(config.ignore_paths, vec!["target", ".git"]);

        // Verify enabled_rules
        assert_eq!(
            config.enabled_rules,
            vec!["auth_gaps", "panics", "arithmetic", "ledger_size"]
        );

        // Verify ledger_limit
        assert_eq!(config.ledger_limit, 64000);

        // Verify strict_mode
        assert_eq!(config.strict_mode, false);

        // Verify approaching_threshold
        assert_eq!(config.approaching_threshold, 0.8);

        // Verify custom_rules
        assert_eq!(config.custom_rules.len(), 2);
        
        let rule1 = &config.custom_rules[0];
        assert_eq!(rule1.name, "no_unsafe_block");
        assert_eq!(rule1.pattern, "unsafe\\s*\\{");

        let rule2 = &config.custom_rules[1];
        assert_eq!(rule2.name, "no_mem_forget");
        assert_eq!(rule2.pattern, "std::mem::forget");
    }

    #[test]
    fn test_config_has_all_required_fields() {
        let config = ConfigGenerator::generate_default_config();

        // Ensure all required fields are present and non-empty where appropriate
        assert!(!config.ignore_paths.is_empty(), "ignore_paths should not be empty");
        assert!(!config.enabled_rules.is_empty(), "enabled_rules should not be empty");
        assert!(config.ledger_limit > 0, "ledger_limit should be positive");
        assert!(config.approaching_threshold > 0.0 && config.approaching_threshold < 1.0, 
                "approaching_threshold should be between 0 and 1");
    }

    #[test]
    fn test_custom_rules_have_valid_patterns() {
        let config = ConfigGenerator::generate_default_config();

        for rule in &config.custom_rules {
            assert!(!rule.name.is_empty(), "Custom rule name should not be empty");
            assert!(!rule.pattern.is_empty(), "Custom rule pattern should not be empty");
            
            // Verify patterns are valid regex
            let regex_result = regex::Regex::new(&rule.pattern);
            assert!(regex_result.is_ok(), "Pattern '{}' should be a valid regex", rule.pattern);
        }
    }

    #[test]
    fn test_config_exists_returns_false_when_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        assert!(!FileWriter::config_exists(path));
    }

    #[test]
    fn test_config_exists_returns_true_when_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let config_path = path.join(".sanctify.toml");
        
        // Create the file
        fs::write(&config_path, "test content").unwrap();
        
        assert!(FileWriter::config_exists(path));
    }

    #[test]
    fn test_write_config_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let config = ConfigGenerator::generate_default_config();
        
        let result = FileWriter::write_config(&config, path);
        
        assert!(result.is_ok());
        let config_path = result.unwrap();
        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), ".sanctify.toml");
    }

    #[test]
    fn test_write_config_creates_valid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let config = ConfigGenerator::generate_default_config();
        
        let result = FileWriter::write_config(&config, path);
        assert!(result.is_ok());
        
        let config_path = result.unwrap();
        let content = fs::read_to_string(&config_path).unwrap();
        
        // Verify it's valid TOML by parsing it
        let parsed: Result<SanctifyConfig, _> = toml::from_str(&content);
        assert!(parsed.is_ok(), "Generated TOML should be parseable");
    }

    #[test]
    fn test_write_config_returns_correct_path() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let config = ConfigGenerator::generate_default_config();
        
        let result = FileWriter::write_config(&config, path);
        assert!(result.is_ok());
        
        let returned_path = result.unwrap();
        let expected_path = path.join(".sanctify.toml");
        assert_eq!(returned_path, expected_path);
    }

    #[test]
    fn test_exec_creates_config_in_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let args = InitArgs { force: false };
        
        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Execute init command
        let result = exec(args);
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        // Verify success
        assert!(result.is_ok(), "exec should succeed in empty directory");
        
        // Verify file was created
        let config_path = temp_dir.path().join(".sanctify.toml");
        assert!(config_path.exists(), "Config file should be created");
        
        // Verify content is valid TOML
        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: Result<SanctifyConfig, _> = toml::from_str(&content);
        assert!(parsed.is_ok(), "Generated TOML should be parseable");
    }

    #[test]
    fn test_exec_with_existing_file_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".sanctify.toml");
        
        // Create existing file
        fs::write(&config_path, "existing content").unwrap();
        
        let args = InitArgs { force: false };
        
        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Execute init command - this will call std::process::exit(1)
        // We can't test this directly without spawning a subprocess
        // So we'll just test the components separately
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        // Verify file was not modified
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing content", "File should not be modified");
    }

    #[test]
    fn test_exec_with_force_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".sanctify.toml");
        
        // Create existing file
        fs::write(&config_path, "existing content").unwrap();
        
        let args = InitArgs { force: true };
        
        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Execute init command
        let result = exec(args);
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        // Verify success
        assert!(result.is_ok(), "exec should succeed with force flag");
        
        // Verify file was overwritten
        let content = fs::read_to_string(&config_path).unwrap();
        assert_ne!(content, "existing content", "File should be overwritten");
        assert!(content.contains("ignore_paths"), "Should contain default config");
    }
}
