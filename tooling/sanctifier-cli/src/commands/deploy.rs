#![allow(dead_code)]

use crate::commands::color as c;
use crate::errors::SanctifierError;
use clap::Args;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VALID_NETWORKS: &[&str] = &["testnet", "futurenet", "mainnet"];

#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Path to the contract to deploy
    #[arg(default_value = ".")]
    pub contract_path: PathBuf,

    /// Target network (testnet, futurenet, mainnet)
    #[arg(short, long, default_value = "testnet")]
    pub network: String,

    /// Soroban secret key (or set SOROBAN_SECRET_KEY env var)
    #[arg(short, long)]
    pub secret_key: Option<String>,

    /// Account ID for the deployer
    #[arg(short, long)]
    pub account_id: Option<String>,

    /// Enable continuous validation after deployment
    #[arg(short, long)]
    pub validate: bool,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub output_format: String,
}

pub struct DeploymentConfig {
    pub wasm_path: PathBuf,
    pub network: String,
    pub secret_key: String,
    pub account_id: Option<String>,
}

pub fn exec(args: DeployArgs) -> anyhow::Result<()> {
    let is_json = args.output_format == "json";

    // Validate network name before doing any I/O.
    if !VALID_NETWORKS.contains(&args.network.as_str()) {
        return Err(SanctifierError::invalid_network(&args.network).into());
    }

    // Validate contract path
    if !args.contract_path.exists() {
        return Err(SanctifierError::path_not_found(&args.contract_path).into());
    }

    // Resolve secret key: explicit flag wins, then env var.
    let secret_key = args
        .secret_key
        .filter(|k| !k.is_empty())
        .or_else(|| {
            std::env::var("SOROBAN_SECRET_KEY")
                .ok()
                .filter(|k| !k.is_empty())
        })
        .ok_or_else(SanctifierError::missing_credentials)?;

    let _ = is_json;

    // Build the contract
    let build_result = build_contract(&args.contract_path, is_json);
    if !build_result {
        std::process::exit(1);
    }

    // Find WASM file
    let wasm_path = find_wasm_file(&args.contract_path);
    if wasm_path.is_none() {
        std::process::exit(1);
    }

    let config = DeploymentConfig {
        wasm_path: wasm_path.unwrap(),
        network: args.network.clone(),
        secret_key: secret_key.clone(),
        account_id: args.account_id,
    };

    // Deploy to testnet
    match deploy_contract(&config, is_json) {
        Ok(contract_id) => {
            if is_json {
                println!(
                    "{{\"status\":\"success\",\"contract_id\":\"{}\",\"network\":\"{}\"}}",
                    contract_id, args.network
                );
            } else {
                println!("{} Contract deployed successfully!", c::green("✅"));
                println!("   Contract ID: {}", c::cyan(&contract_id));
                println!("   Network: {}", args.network);
            }

            // Optionally run validation
            if args.validate {
                if !is_json {
                    println!("{} Running continuous validation...", c::blue("🔍"));
                }
                validate_deployment(&contract_id, &args.network, is_json)?;
            }

            Ok(())
        }
        Err(e) => {
            if is_json {
                println!("{{\"status\":\"error\",\"message\":\"{}\"}} ", e);
            } else {
                eprintln!("{} Deployment failed: {}", c::red("❌"), e);
            }
            std::process::exit(1);
        }
    }
}

fn build_contract(contract_path: &PathBuf, is_json: bool) -> bool {
    if !is_json {
        println!("{} Building contract...", c::bright_yellow("🔨"));
    }

    let cargo_toml = contract_path.join("Cargo.toml");
    if !cargo_toml.exists() {
        eprintln!(
            "{} Error: Cargo.toml not found in {}",
            c::red("❌"),
            contract_path.display()
        );
        return false;
    }

    let output = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(contract_path)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                if !is_json {
                    println!("{} Contract built successfully", c::green("✓"));
                }
                true
            } else {
                eprintln!(
                    "{} Build failed:\n{}",
                    c::red("❌"),
                    String::from_utf8_lossy(&result.stderr)
                );
                false
            }
        }
        Err(e) => {
            eprintln!("{} Failed to run cargo build: {}", c::red("❌"), e);
            false
        }
    }
}

fn find_wasm_file(contract_path: &Path) -> Option<PathBuf> {
    let target_dir = contract_path.join("target/wasm32-unknown-unknown/release");

    if target_dir.exists() {
        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "wasm" {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

fn deploy_contract(config: &DeploymentConfig, is_json: bool) -> Result<String, String> {
    if !is_json {
        println!(
            "{} Deploying to {}...",
            c::bright_cyan("📦"),
            config.network
        );
    }

    let output = Command::new("soroban")
        .args([
            "contract",
            "deploy",
            "--wasm",
            config.wasm_path.to_str().unwrap(),
            "--source",
            &config.secret_key,
            "--network",
            &config.network,
        ])
        .output()
        .map_err(|e| format!("Failed to deploy: {}", e))?;

    if output.status.success() {
        let contract_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(contract_id)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn validate_deployment(contract_id: &str, network: &str, is_json: bool) -> anyhow::Result<()> {
    if !is_json {
        println!(
            "{} Validating contract {} on {}...",
            c::green("✓"),
            c::cyan(contract_id),
            network
        );
    }

    // Invoke health check
    let output = Command::new("soroban")
        .args([
            "contract",
            "invoke",
            "--id",
            contract_id,
            "--network",
            network,
            "--",
            "health_check",
        ])
        .output()?;

    if output.status.success() {
        if !is_json {
            println!("{} Validation passed!", c::green("✅"));
        }
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        if !is_json {
            eprintln!("{} Validation failed: {}", c::red("❌"), error);
        }
        Err(anyhow::anyhow!("Validation failed: {}", error))
    }
}
