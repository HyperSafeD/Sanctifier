//! `sanctifier harness` — generate native `afl.rs` / `honggfuzz` fuzz-target
//! scaffolds from static analysis of a Soroban contract's public ABI.
//!
//! This bridges *static* analysis (the AST-level function/parameter
//! extraction performed by [`sanctifier_core::harness_spec`]) to *dynamic*
//! analysis: each public, non-reserved `#[contractimpl]` function becomes a
//! standalone fuzz target that
//!
//! 1. derives an `Arbitrary` input struct whose fields are
//!    `<ParamType as SorobanArbitrary>::Prototype` — the pattern
//!    documented by `soroban_sdk::testutils::arbitrary` for fuzzing
//!    host-managed contract types, and
//! 2. registers the contract, builds a client, converts each prototype into
//!    its real Soroban type with `.into_val(&env)`, and invokes
//!    `client.try_<fn>(..)`.
//!
//! Generated scaffolds are self-contained, workspace-excluded mini-crates
//! (mirroring the `contracts/*/fuzz/Cargo.toml` convention already used in
//! this repository for `cargo-fuzz`/libFuzzer harnesses) so they can be
//! built independently with `cargo afl build` / `cargo hfuzz build` without
//! disturbing the analyzed contract's own workspace.

use anyhow::Context;
use clap::{Args, ValueEnum};
use sanctifier_core::harness_spec::{self, HarnessContract, HarnessFunction};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::color as c;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuzzTarget {
    /// Generate an `afl.rs`-based scaffold only.
    Afl,
    /// Generate a `honggfuzz`-based scaffold only.
    Honggfuzz,
    /// Generate both scaffolds (default).
    Both,
}

#[derive(Args, Debug)]
pub struct HarnessArgs {
    /// Path to the Rust source file containing one or more Soroban contracts
    pub path: PathBuf,

    /// Output directory for the generated fuzz-harness scaffold(s)
    #[arg(short, long, default_value = "fuzz-harness")]
    pub output: PathBuf,

    /// Which fuzzing backend(s) to scaffold
    #[arg(short, long, value_enum, default_value_t = FuzzTarget::Both)]
    pub target: FuzzTarget,

    /// Only generate a harness for this function name (default: all public functions)
    #[arg(long)]
    pub function: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Afl,
    Honggfuzz,
}

impl Backend {
    fn dir_name(self) -> &'static str {
        match self {
            Backend::Afl => "afl",
            Backend::Honggfuzz => "honggfuzz",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Backend::Afl => "AFL (afl.rs)",
            Backend::Honggfuzz => "honggfuzz",
        }
    }

    fn dependency_line(self) -> &'static str {
        match self {
            Backend::Afl => "afl = \"0.15\"",
            Backend::Honggfuzz => "honggfuzz = \"0.5\"",
        }
    }

    fn use_line(self) -> &'static str {
        match self {
            Backend::Afl => "use afl::fuzz;",
            Backend::Honggfuzz => "use honggfuzz::fuzz;",
        }
    }

    fn run_instructions(self, bin_name: &str) -> String {
        match self {
            Backend::Afl => format!(
                "//! Run with:\n//!   cargo afl build\n//!   cargo afl fuzz -i in -o out target/debug/{bin_name}"
            ),
            Backend::Honggfuzz => format!(
                "//! Run with:\n//!   cargo hfuzz build\n//!   cargo hfuzz run {bin_name}"
            ),
        }
    }
}

struct CrateInfo {
    /// Cargo package name (as declared in `[package] name`), used verbatim
    /// as the `path = ...` dependency key.
    name: String,
    /// Rust identifier form of `name` (hyphens replaced with underscores),
    /// used in `use` statements.
    ident: String,
    /// Absolute path to the crate root directory (containing `Cargo.toml`).
    dir: PathBuf,
}

/// Walk upward from `source_path`'s parent directory looking for the
/// nearest `Cargo.toml`, returning its declared package name and location.
///
/// Returns `None` (rather than an error) when no manifest is found, since a
/// harness can still be generated — just with a `// TODO` placeholder
/// import — for a source file that is not part of a Cargo crate on disk.
fn locate_crate_info(source_path: &Path) -> Option<CrateInfo> {
    let mut dir = source_path.parent()?.to_path_buf();
    if dir.as_os_str().is_empty() {
        dir = PathBuf::from(".");
    }
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            let content = fs::read_to_string(&candidate).ok()?;
            let doc: toml::Value = content.parse().ok()?;
            let name = doc.get("package")?.get("name")?.as_str()?.to_string();
            let ident = name.replace('-', "_");
            let abs_dir = fs::canonicalize(&dir).unwrap_or(dir.clone());
            return Some(CrateInfo {
                name,
                ident,
                dir: abs_dir,
            });
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// `MyContract` -> `my_contract`.
fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else if ch == '-' {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    out
}

fn harness_bin_name(struct_name: &str, fn_name: &str) -> String {
    format!("{}_{}", to_snake_case(struct_name), fn_name)
}

pub fn exec(args: HarnessArgs) -> anyhow::Result<()> {
    let source = fs::read_to_string(&args.path)
        .with_context(|| format!("failed to read {}", args.path.display()))?;

    let parsed = sanctifier_core::parser::parse_source(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", args.path.display(), e))?;

    let mut contracts = harness_spec::extract_harness_contracts(&parsed.file);

    if let Some(filter) = &args.function {
        for contract in &mut contracts {
            contract.functions.retain(|f| &f.name == filter);
        }
        contracts.retain(|c| !c.functions.is_empty());
    }

    if contracts.is_empty() {
        println!(
            "{} No fuzzable public contract functions found in {}",
            c::yellow_warning(),
            args.path.display()
        );
        return Ok(());
    }

    let crate_info = locate_crate_info(&args.path);
    if crate_info.is_none() {
        println!(
            "{} Could not locate a Cargo.toml above {} — generated scaffolds will contain a TODO placeholder import instead of a path dependency.",
            c::yellow_warning(),
            args.path.display()
        );
    }

    let backends: &[Backend] = match args.target {
        FuzzTarget::Afl => &[Backend::Afl],
        FuzzTarget::Honggfuzz => &[Backend::Honggfuzz],
        FuzzTarget::Both => &[Backend::Afl, Backend::Honggfuzz],
    };

    let mut total_generated = 0usize;
    for &backend in backends {
        total_generated +=
            generate_backend_scaffold(backend, &contracts, crate_info.as_ref(), &args.output)?;
    }

    let fn_count: usize = contracts.iter().map(|c| c.functions.len()).sum();
    println!(
        "{} Generated {} fuzz target(s) ({} function(s) x {} backend(s)) in {}",
        c::green_check(),
        total_generated,
        fn_count,
        backends.len(),
        args.output.display()
    );
    Ok(())
}

fn generate_backend_scaffold(
    backend: Backend,
    contracts: &[HarnessContract],
    crate_info: Option<&CrateInfo>,
    output_root: &Path,
) -> anyhow::Result<usize> {
    let backend_dir = output_root.join(backend.dir_name());
    let bin_dir = backend_dir.join("src").join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    let mut bin_entries: Vec<(String, String)> = Vec::new();

    for contract in contracts {
        for function in &contract.functions {
            let bin_name = harness_bin_name(&contract.struct_name, &function.name);
            let file_name = format!("{bin_name}.rs");
            let source = render_harness_source(backend, contract, function, crate_info);
            let file_path = bin_dir.join(&file_name);
            fs::write(&file_path, source)
                .with_context(|| format!("failed to write {}", file_path.display()))?;
            bin_entries.push((bin_name, format!("src/bin/{file_name}")));
        }
    }

    let manifest = render_manifest(backend, crate_info, &bin_entries);
    let manifest_path = backend_dir.join("Cargo.toml");
    fs::write(&manifest_path, manifest)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    Ok(bin_entries.len())
}

fn render_manifest(
    backend: Backend,
    crate_info: Option<&CrateInfo>,
    bins: &[(String, String)],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "[package]");
    let _ = writeln!(out, "name = \"fuzz-harness-{}\"", backend.dir_name());
    let _ = writeln!(out, "version = \"0.0.1\"");
    let _ = writeln!(out, "edition = \"2021\"");
    let _ = writeln!(out, "publish = false");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "# Excludes this scaffold from any parent Cargo workspace, mirroring the"
    );
    let _ = writeln!(
        out,
        "# convention used by contracts/*/fuzz crates in this repository."
    );
    let _ = writeln!(out, "[workspace]");
    let _ = writeln!(out);
    let _ = writeln!(out, "[dependencies]");
    let _ = writeln!(out, "{}", backend.dependency_line());
    let _ = writeln!(
        out,
        "arbitrary = {{ version = \"1\", features = [\"derive\"] }}"
    );
    let _ = writeln!(
        out,
        "soroban-sdk = {{ version = \"21.7.0\", features = [\"testutils\"] }}"
    );
    match crate_info {
        Some(info) => {
            let _ = writeln!(
                out,
                "{} = {{ path = {:?}, features = [\"testutils\"] }}",
                info.name, info.dir
            );
        }
        None => {
            let _ = writeln!(
                out,
                "# TODO: point this at your contract crate, enabling a `testutils` feature"
            );
            let _ = writeln!(
                out,
                "# that turns on `soroban-sdk/testutils` (required for `SorobanArbitrary`"
            );
            let _ = writeln!(out, "# to be derived on your #[contracttype] types).");
            let _ = writeln!(
                out,
                "# your-contract = {{ path = \"../..\", features = [\"testutils\"] }}"
            );
        }
    }
    let _ = writeln!(out);

    for (bin_name, path) in bins {
        let _ = writeln!(out, "[[bin]]");
        let _ = writeln!(out, "name = {bin_name:?}");
        let _ = writeln!(out, "path = {path:?}");
        let _ = writeln!(out, "test = false");
        let _ = writeln!(out, "doc = false");
        let _ = writeln!(out);
    }

    out
}

fn render_harness_source(
    backend: Backend,
    contract: &HarnessContract,
    function: &HarnessFunction,
    crate_info: Option<&CrateInfo>,
) -> String {
    let contract_ty = &contract.struct_name;
    let client_ty = format!("{contract_ty}Client");
    let fn_name = &function.name;
    let bin_name = harness_bin_name(contract_ty, fn_name);

    let mut out = String::new();

    // ── Doc header ─────────────────────────────────────────────────────────
    let _ = writeln!(
        out,
        "//! Auto-generated {} fuzz harness for `{contract_ty}::{fn_name}`.",
        backend.display_name()
    );
    let _ = writeln!(out, "//!");
    let _ = writeln!(
        out,
        "//! Generated by `sanctifier harness` — bridges static analysis to dynamic"
    );
    let _ = writeln!(
        out,
        "//! analysis. Review the imports and call site below before fuzzing."
    );
    let _ = writeln!(out, "//!");
    let _ = writeln!(out, "{}", backend.run_instructions(&bin_name));
    let _ = writeln!(out);

    // ── Imports ────────────────────────────────────────────────────────────
    let _ = writeln!(
        out,
        "use soroban_sdk::testutils::arbitrary::{{Arbitrary, SorobanArbitrary}};"
    );
    let _ = writeln!(out, "use soroban_sdk::{{Env, IntoVal}};");
    let _ = writeln!(out, "{}", backend.use_line());
    let _ = writeln!(out);
    match crate_info {
        Some(info) => {
            let _ = writeln!(out, "use {}::{{{contract_ty}, {client_ty}}};", info.ident);
        }
        None => {
            let _ = writeln!(out, "// TODO: could not auto-detect the contract crate.");
            let _ = writeln!(
                out,
                "// use your_contract_crate::{{{contract_ty}, {client_ty}}};"
            );
        }
    }
    let _ = writeln!(out);

    // ── Fuzz input struct ──────────────────────────────────────────────────
    let _ = writeln!(out, "#[derive(Debug, Arbitrary)]");
    if function.params.is_empty() {
        let _ = writeln!(out, "struct FuzzInput;");
    } else {
        let _ = writeln!(out, "struct FuzzInput {{");
        for p in &function.params {
            let _ = writeln!(
                out,
                "    {}: <{} as SorobanArbitrary>::Prototype,",
                p.name, p.ty_tokens
            );
        }
        let _ = writeln!(out, "}}");
    }
    let _ = writeln!(out);

    // ── Entry point ────────────────────────────────────────────────────────
    let _ = writeln!(out, "fn main() {{");
    match backend {
        Backend::Afl => {
            let _ = writeln!(out, "    fuzz!(|input: FuzzInput| {{");
        }
        Backend::Honggfuzz => {
            let _ = writeln!(out, "    loop {{");
            let _ = writeln!(out, "        fuzz!(|input: FuzzInput| {{");
        }
    }
    let indent = match backend {
        Backend::Afl => "        ",
        Backend::Honggfuzz => "            ",
    };
    let _ = writeln!(out, "{indent}let env = Env::default();");
    let _ = writeln!(out, "{indent}env.mock_all_auths();");
    let _ = writeln!(
        out,
        "{indent}let contract_id = env.register_contract(None, {contract_ty});"
    );
    let _ = writeln!(
        out,
        "{indent}let client = {client_ty}::new(&env, &contract_id);"
    );
    if !function.params.is_empty() {
        let _ = writeln!(out);
        for p in &function.params {
            let _ = writeln!(
                out,
                "{indent}let {}: {} = input.{}.into_val(&env);",
                p.name, p.ty_tokens, p.name
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{indent}// `try_{fn_name}` reports contract errors as `Err` instead of panicking,"
    );
    let _ = writeln!(
        out,
        "{indent}// so the fuzzer only treats host-level panics as crashes. Switch to"
    );
    let _ = writeln!(
        out,
        "{indent}// `{fn_name}` directly if you want expected `Err` returns to count too."
    );
    let call_args = function
        .params
        .iter()
        .map(|p| format!("&{}", p.name))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "{indent}let _ = client.try_{fn_name}({call_args});");
    match backend {
        Backend::Afl => {
            let _ = writeln!(out, "    }});");
        }
        Backend::Honggfuzz => {
            let _ = writeln!(out, "        }});");
            let _ = writeln!(out, "    }}");
        }
    }
    let _ = writeln!(out, "}}");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_snake_case_converts_pascal_case() {
        assert_eq!(to_snake_case("MyContract"), "my_contract");
        assert_eq!(to_snake_case("Token"), "token");
        assert_eq!(to_snake_case("ZKVerifier"), "z_k_verifier");
    }

    #[test]
    fn harness_bin_name_combines_contract_and_function() {
        assert_eq!(harness_bin_name("Token", "transfer"), "token_transfer");
    }

    #[test]
    fn render_manifest_without_crate_info_has_todo_placeholder() {
        let manifest = render_manifest(Backend::Afl, None, &[]);
        assert!(manifest.contains("afl = \"0.15\""));
        assert!(manifest.contains("[workspace]"));
        assert!(manifest.contains("TODO"));
    }

    #[test]
    fn render_manifest_with_crate_info_has_path_dependency() {
        let info = CrateInfo {
            name: "my-contract".to_string(),
            ident: "my_contract".to_string(),
            dir: PathBuf::from("/tmp/my-contract"),
        };
        let manifest = render_manifest(
            Backend::Honggfuzz,
            Some(&info),
            &[(
                "my_contract_transfer".to_string(),
                "src/bin/my_contract_transfer.rs".to_string(),
            )],
        );
        assert!(manifest.contains("honggfuzz = \"0.5\""));
        assert!(manifest.contains("my-contract = { path"));
        assert!(manifest.contains("testutils"));
        assert!(manifest.contains("[[bin]]"));
        assert!(manifest.contains("name = \"my_contract_transfer\""));
    }

    #[test]
    fn render_harness_source_afl_contains_expected_shape() {
        let contract = HarnessContract {
            struct_name: "Token".to_string(),
            functions: vec![],
        };
        let function = HarnessFunction {
            name: "transfer".to_string(),
            params: vec![
                harness_spec::HarnessParam {
                    name: "from".to_string(),
                    ty_tokens: "Address".to_string(),
                },
                harness_spec::HarnessParam {
                    name: "amount".to_string(),
                    ty_tokens: "i128".to_string(),
                },
            ],
        };
        let src = render_harness_source(Backend::Afl, &contract, &function, None);
        assert!(src.contains("use afl::fuzz;"));
        assert!(src.contains("struct FuzzInput {"));
        assert!(src.contains("from: <Address as SorobanArbitrary>::Prototype,"));
        assert!(src.contains("amount: <i128 as SorobanArbitrary>::Prototype,"));
        assert!(src.contains("let contract_id = env.register_contract(None, Token);"));
        assert!(src.contains("let client = TokenClient::new(&env, &contract_id);"));
        assert!(src.contains("let from: Address = input.from.into_val(&env);"));
        assert!(src.contains("let _ = client.try_transfer(&from, &amount);"));
        assert!(src.contains("fuzz!(|input: FuzzInput| {"));
    }

    #[test]
    fn render_harness_source_honggfuzz_wraps_in_loop() {
        let contract = HarnessContract {
            struct_name: "Counter".to_string(),
            functions: vec![],
        };
        let function = HarnessFunction {
            name: "increment".to_string(),
            params: vec![],
        };
        let src = render_harness_source(Backend::Honggfuzz, &contract, &function, None);
        assert!(src.contains("use honggfuzz::fuzz;"));
        assert!(src.contains("loop {"));
        assert!(src.contains("struct FuzzInput;"));
        assert!(src.contains("let _ = client.try_increment();"));
    }

    #[test]
    fn to_snake_case_handles_hyphens() {
        assert_eq!(to_snake_case("my-contract"), "my_contract");
    }
}
