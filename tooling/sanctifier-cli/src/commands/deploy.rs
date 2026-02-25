use clap::Args;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Path to the contract directory or Cargo.toml
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Soroban network to deploy to (testnet/mainnet/local)
    #[arg(short, long, default_value = "testnet")]
    pub network: String,

    /// Optional secret key (falls back to SOROBAN_SECRET_KEY environment variable)
    #[arg(short, long)]
    pub secret: Option<String>,

    /// Optional explicit package name (if Cargo.toml package name differs)
    #[arg(short, long)]
    pub package: Option<String>,
}

pub fn read_package_name(cargo_toml: &Path) -> anyhow::Result<String> {
    let content = fs::read_to_string(cargo_toml)?;
    let doc: toml::Value = toml::from_str(&content)?;
    if let Some(pkg) = doc.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
        Ok(pkg.to_string())
    } else {
        anyhow::bail!("Failed to read package.name from Cargo.toml")
    }
}

/// Locate the compiled wasm artifact for a given project directory and package name.
/// Returns Some(path) if the artifact exists, None otherwise.
pub fn find_wasm_artifact(project_dir: &Path, package: &str) -> Option<PathBuf> {
    let wasm_name = package.replace('-', "_") + ".wasm";
    let wasm_path = project_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(&wasm_name);
    if wasm_path.exists() {
        Some(wasm_path)
    } else {
        None
    }
}

pub fn exec(args: DeployArgs) -> anyhow::Result<()> {
    let path = args.path;

    // Resolve project directory and Cargo.toml
    let cargo_toml = if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path.to_path_buf()
    };

    if !cargo_toml.exists() {
        eprintln!("{} Error: Cargo.toml not found at {:?}", "✗".red(), cargo_toml);
        std::process::exit(1);
    }

    let package = match args.package {
        Some(p) => p,
        None => read_package_name(&cargo_toml)?,
    };

    // Compute expected wasm filename (replace '-' with '_' to match rust crate file)
    let wasm_name = package.replace('-', "_") + ".wasm";
    let project_dir = if cargo_toml.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
        cargo_toml.parent().unwrap().to_path_buf()
    } else {
        PathBuf::from(".")
    };

    println!("{} Building contract...", "🔨".blue());

    // Build WASM
    let status = Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--release")
        .current_dir(&project_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("cargo build failed; please ensure the contract builds correctly")
    }

    let wasm_path = project_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(&wasm_name);

    if !wasm_path.exists() {
        eprintln!(
            "{} Error: WASM file not found at {}",
            "✗".red(),
            wasm_path.display()
        );
        anyhow::bail!("WASM artifact not found after build")
    }

    // Determine secret
    let secret = match args.secret {
        Some(s) => s,
        None => env::var("SOROBAN_SECRET_KEY").unwrap_or_default(),
    };

    if secret.is_empty() {
        eprintln!(
            "{} Error: No secret key provided. Set --secret or SOROBAN_SECRET_KEY env var.",
            "✗".red()
        );
        std::process::exit(1);
    }

    // Verify soroban CLI is available
    if which::which("soroban").is_err() {
        eprintln!(
            "{} Error: `soroban` CLI not found in PATH. Please install with `cargo install --locked soroban-cli`.",
            "✗".red()
        );
        std::process::exit(1);
    }

    println!("{} Deploying to {}...", "🚀".green(), args.network);

    // Call soroban CLI: soroban contract deploy --wasm <wasm> --source <secret> --network <network>
    let output = Command::new("soroban")
        .arg("contract")
        .arg("deploy")
        .arg("--wasm")
        .arg(wasm_path.to_string_lossy().to_string())
        .arg("--source")
        .arg(secret)
        .arg("--network")
        .arg(args.network)
        .output()?;

    if !output.status.success() {
        eprintln!("{} Deployment failed:", "✗".red());
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr);
        anyhow::bail!("soroban deploy failed")
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{} Deployment finished.", "✅".green());
    println!("{}", stdout);

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};

    #[test]
    fn test_read_package_name_and_find_wasm_artifact() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path();

        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml_contents = r#"
[package]
name = "vulnerable-contract"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(&cargo_toml_path, cargo_toml_contents).unwrap();

        // Create fake wasm artifact path
        let wasm_dir = project_dir
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release");
        fs::create_dir_all(&wasm_dir).unwrap();
        let wasm_file = wasm_dir.join("vulnerable_contract.wasm");
        File::create(&wasm_file).unwrap();

        // Read package name
        let pkg = read_package_name(&cargo_toml_path).expect("should read package name");
        assert_eq!(pkg, "vulnerable-contract");

        // Find wasm artifact
        let found = find_wasm_artifact(project_dir, &pkg);
        assert!(found.is_some(), "WASM artifact should be found");
        let found_path = found.unwrap();
        assert!(found_path.ends_with("vulnerable_contract.wasm"));
    }
}
