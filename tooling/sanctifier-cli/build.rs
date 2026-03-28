use regex::Regex;
use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Validate version at compile time
    let version = env!("CARGO_PKG_VERSION");
    let version_regex = Regex::new(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$")?;
    if !version_regex.is_match(version) {
        panic!(
            "CARGO_PKG_VERSION '{}' is not a valid semver tag pattern",
            version
        );
    }

    // 2. Embed build metadata
    EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .all_sysinfo()
        .emit()?;

    Ok(())
}
