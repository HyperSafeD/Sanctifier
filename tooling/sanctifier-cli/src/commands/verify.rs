#![allow(dead_code)]
use anyhow::{bail, Context};
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Path to the local Soroban contract directory (must contain Cargo.toml)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// On-chain contract ID to fetch the deployed WASM from
    #[arg(long)]
    pub contract_id: String,

    /// Stellar/Soroban network passphrase or alias (e.g. "testnet", "mainnet")
    #[arg(long, default_value = "testnet")]
    pub network: String,

    /// Path to a pre-fetched on-chain WASM file (skips network fetch)
    #[arg(long)]
    pub wasm_file: Option<PathBuf>,
}

pub fn exec(args: VerifyArgs) -> anyhow::Result<()> {
    println!("{}", "sanctifier verify — bytecode verification".bold());
    println!();

    // Step 1: Build local WASM
    let local_wasm = build_local_wasm(&args.path)?;

    // Step 2: Obtain on-chain WASM (from file or network)
    let remote_wasm_path = match &args.wasm_file {
        Some(p) => p.clone(),
        None => fetch_onchain_wasm(&args.contract_id, &args.network)?,
    };

    // Step 3: Read and hash both files
    let local_bytes = std::fs::read(&local_wasm)
        .with_context(|| format!("failed to read local WASM: {}", local_wasm.display()))?;
    let remote_bytes = std::fs::read(&remote_wasm_path).with_context(|| {
        format!(
            "failed to read on-chain WASM: {}",
            remote_wasm_path.display()
        )
    })?;

    let local_hash = sha256_hex(&local_bytes);
    let remote_hash = sha256_hex(&remote_bytes);

    info!(target: "sanctifier", local_hash = %local_hash, remote_hash = %remote_hash, "bytecode hashes");

    println!(
        "  Local  WASM : {} bytes  sha256={}",
        local_bytes.len(),
        &local_hash[..16]
    );
    println!(
        "  Remote WASM : {} bytes  sha256={}",
        remote_bytes.len(),
        &remote_hash[..16]
    );
    println!();

    if local_hash == remote_hash {
        println!(
            "{} Source matches the on-chain deployment.",
            "✓".green().bold()
        );
        println!("  Full hash: {}", local_hash.dimmed());
        Ok(())
    } else {
        println!(
            "{} MISMATCH — local source does NOT match the on-chain WASM.",
            "✗".red().bold()
        );
        println!("  Local  sha256: {}", local_hash);
        println!("  Remote sha256: {}", remote_hash);
        bail!(
            "bytecode mismatch detected for contract {}",
            args.contract_id
        );
    }
}

/// Build the contract in release mode and return the path to the produced WASM.
fn build_local_wasm(contract_path: &Path) -> anyhow::Result<PathBuf> {
    println!("  Building local WASM …");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
        ])
        .arg(contract_path.join("Cargo.toml"))
        .status()
        .context("failed to invoke `cargo build`")?;

    if !status.success() {
        bail!("cargo build failed — check the contract source for errors");
    }

    // Locate the WASM in target/wasm32-unknown-unknown/release/
    let target_dir = contract_path.join("target/wasm32-unknown-unknown/release");
    let wasm: Vec<_> = std::fs::read_dir(&target_dir)
        .with_context(|| format!("cannot read target dir {}", target_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "wasm").unwrap_or(false))
        .collect();

    match wasm.len() {
        0 => bail!("no .wasm artifact found under {}", target_dir.display()),
        1 => Ok(wasm[0].path()),
        _ => {
            // Pick the largest one (most likely the contract, not a dependency stub)
            let best = wasm
                .iter()
                .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .unwrap();
            Ok(best.path())
        }
    }
}

/// Use the Stellar CLI to download the on-chain WASM for `contract_id`.
fn fetch_onchain_wasm(contract_id: &str, network: &str) -> anyhow::Result<PathBuf> {
    let out_path = std::env::temp_dir().join(format!("{contract_id}.wasm"));
    println!("  Fetching on-chain WASM for {contract_id} …");

    // Try `stellar` first, fall back to `soroban`
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
                return Ok(out_path);
            }
        }
    }

    bail!(
        "could not fetch on-chain WASM — ensure `stellar` or `soroban` CLI is installed \
         and you are authenticated to the '{network}' network"
    );
}

fn sha256_hex(data: &[u8]) -> String {
    let bytes = Sha256::digest(data);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Minimal SHA-256 implementation (RFC 6234) — no external dependencies.
struct Sha256;

impl Sha256 {
    fn digest(data: &[u8]) -> [u8; 32] {
        // SHA-256 constants
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

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];

        // Pre-processing: adding padding bits
        let bit_len = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in msg.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }

        let mut out = [0u8; 32];
        for (i, word) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469348423f656ab8cf31
        let digest = Sha256::digest(b"abc");
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        assert!(hex.starts_with("ba7816bf"), "got: {hex}");
    }

    #[test]
    fn sha256_empty() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let digest = Sha256::digest(b"");
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        assert!(hex.starts_with("e3b0c442"), "got: {hex}");
    }
}
