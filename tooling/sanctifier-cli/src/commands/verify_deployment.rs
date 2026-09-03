#![allow(dead_code)]

use crate::commands::color as c;
use anyhow::{bail, Context};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Args, Debug)]
pub struct VerifyDeploymentArgs {
    /// On-chain contract ID to verify
    #[arg(long)]
    pub contract_id: String,

    /// Stellar network alias or passphrase (e.g. "testnet", "mainnet")
    #[arg(long, default_value = "testnet")]
    pub network: String,

    /// Path to local contract source to build and compare (defaults to current dir)
    #[arg(long, default_value = ".")]
    pub source: PathBuf,

    /// Skip local build; compare against this pre-computed SHA-256 hex digest
    #[arg(long)]
    pub expected_hash: Option<String>,

    /// Output format: text (default) or json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Serialize)]
struct VerifyDeploymentReport {
    contract_id: String,
    network: String,
    local_hash: String,
    remote_hash: String,
    match_result: bool,
}

pub fn exec(args: VerifyDeploymentArgs) -> anyhow::Result<()> {
    let is_json = args.format == "json";

    if !is_json {
        println!("{}", c::bold("sanctifier verify-deployment"));
        println!("  Contract : {}", args.contract_id);
        println!("  Network  : {}", args.network);
        println!();
    }

    // Obtain the local WASM hash — either from a build or a supplied digest
    let local_hash = match &args.expected_hash {
        Some(h) => {
            if !is_json {
                println!("  Using supplied expected hash (skipping local build).");
            }
            h.clone()
        }
        None => {
            if !is_json {
                println!("  Building local WASM …");
            }
            build_and_hash(&args.source)?
        }
    };

    // Fetch remote WASM hash via stellar/soroban CLI
    if !is_json {
        println!("  Fetching remote WASM hash for {} …", args.contract_id);
    }
    let remote_hash = fetch_remote_hash(&args.contract_id, &args.network)?;

    let matched = local_hash == remote_hash;

    if is_json {
        let report = VerifyDeploymentReport {
            contract_id: args.contract_id.clone(),
            network: args.network.clone(),
            local_hash,
            remote_hash,
            match_result: matched,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        if !matched {
            bail!(
                "deployment verification failed for contract {}",
                args.contract_id
            );
        }
        return Ok(());
    }

    println!("  Local  sha256: {}", local_hash);
    println!("  Remote sha256: {}", remote_hash);
    println!();

    if matched {
        println!(
            "{} Deployment verified — on-chain WASM matches local source.",
            c::green_bold("✓")
        );
        Ok(())
    } else {
        println!(
            "{} MISMATCH — deployed WASM does NOT match local source.",
            c::red_bold("✗")
        );
        bail!(
            "deployment verification failed: hash mismatch for contract {}",
            args.contract_id
        );
    }
}

/// Build the contract in release mode and return its SHA-256 hex digest.
fn build_and_hash(source: &std::path::Path) -> anyhow::Result<String> {
    // Try `stellar contract build` first (Stellar CLI ≥ 0.9), fall back to cargo directly
    let stellar_status = Command::new("stellar")
        .args(["contract", "build"])
        .current_dir(source)
        .status();

    let built = match stellar_status {
        Ok(s) if s.success() => true,
        _ => {
            let status = Command::new("cargo")
                .args([
                    "build",
                    "--release",
                    "--target",
                    "wasm32v1-none",
                    "--manifest-path",
                ])
                .arg(source.join("Cargo.toml"))
                .status()
                .context("failed to invoke `cargo build`")?;
            status.success()
        }
    };

    if !built {
        bail!("contract build failed — check the source for errors");
    }

    let wasm = find_wasm(source)?;
    let bytes = std::fs::read(&wasm)
        .with_context(|| format!("failed to read WASM artifact: {}", wasm.display()))?;
    Ok(sha256_hex(&bytes))
}

fn find_wasm(source: &std::path::Path) -> anyhow::Result<PathBuf> {
    for target_triple in &["wasm32v1-none", "wasm32-unknown-unknown"] {
        let dir = source.join(format!("target/{}/release", target_triple));
        if !dir.exists() {
            continue;
        }
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .with_context(|| format!("cannot read {}", dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "wasm").unwrap_or(false))
            .collect();
        if !entries.is_empty() {
            let best = entries
                .iter()
                .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .unwrap();
            return Ok(best.path());
        }
    }
    bail!(
        "no .wasm artifact found under {}/target/*/release/",
        source.display()
    );
}

/// Fetch the WASM for `contract_id` from the network and return its SHA-256 hash.
fn fetch_remote_hash(contract_id: &str, network: &str) -> anyhow::Result<String> {
    let out_path = std::env::temp_dir().join(format!("sanctifier-{contract_id}.wasm"));

    for cli in &["stellar", "soroban"] {
        let status = Command::new(cli)
            .args([
                "contract",
                "fetch",
                "--id",
                contract_id,
                "--network",
                network,
                "--output-file",
            ])
            .arg(&out_path)
            .status();

        if let Ok(s) = status {
            if s.success() {
                let bytes = std::fs::read(&out_path).with_context(|| {
                    format!("failed to read fetched WASM: {}", out_path.display())
                })?;
                let _ = std::fs::remove_file(&out_path);
                return Ok(sha256_hex(&bytes));
            }
        }
    }

    bail!(
        "could not fetch on-chain WASM — ensure `stellar` or `soroban` CLI is installed \
         and authenticated to the '{}' network",
        network
    );
}

fn sha256_hex(data: &[u8]) -> String {
    use std::num::Wrapping as W;

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [W<u32>; 8] = [
        W(0x6a09e667),
        W(0xbb67ae85),
        W(0x3c6ef372),
        W(0xa54ff53a),
        W(0x510e527f),
        W(0x9b05688c),
        W(0x1f83d9ab),
        W(0x5be0cd19),
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [W(0u32); 64];
        for i in 0..16 {
            w[i] = W(u32::from_be_bytes(
                chunk[i * 4..i * 4 + 4].try_into().unwrap(),
            ));
        }
        for i in 16..64 {
            let s0 =
                w[i - 15].0.rotate_right(7) ^ w[i - 15].0.rotate_right(18) ^ (w[i - 15].0 >> 3);
            let s1 = w[i - 2].0.rotate_right(17) ^ w[i - 2].0.rotate_right(19) ^ (w[i - 2].0 >> 10);
            w[i] = w[i - 16] + W(s0) + w[i - 7] + W(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.0.rotate_right(6) ^ e.0.rotate_right(11) ^ e.0.rotate_right(25);
            let ch = (e.0 & f.0) ^ ((!e.0) & g.0);
            let temp1 = hh + W(s1) + W(ch) + W(K[i]) + w[i];
            let s0 = a.0.rotate_right(2) ^ a.0.rotate_right(13) ^ a.0.rotate_right(22);
            let maj = (a.0 & b.0) ^ (a.0 & c.0) ^ (b.0 & c.0);
            let temp2 = W(s0) + W(maj);
            hh = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = b;
            b = a;
            a = temp1 + temp2;
        }
        h[0] += a;
        h[1] += b;
        h[2] += c;
        h[3] += d;
        h[4] += e;
        h[5] += f;
        h[6] += g;
        h[7] += hh;
    }

    h.iter()
        .flat_map(|w| w.0.to_be_bytes())
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        let hex = sha256_hex(b"abc");
        assert!(hex.starts_with("ba7816bf"), "got: {hex}");
    }

    #[test]
    fn sha256_empty() {
        let hex = sha256_hex(b"");
        assert!(hex.starts_with("e3b0c442"), "got: {hex}");
    }
}
