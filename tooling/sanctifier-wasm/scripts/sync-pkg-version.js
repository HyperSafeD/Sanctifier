#!/usr/bin/env node
// Synchronise pkg/package.json version with the version in Cargo.toml.
//
// Run after `wasm-pack build` in CI to ensure the published npm package
// always carries the same semver as the Rust crate.
//
// Usage: node scripts/sync-pkg-version.js

const fs = require("fs");
const path = require("path");

const cargoPath = path.resolve(__dirname, "..", "Cargo.toml");
const pkgPath = path.resolve(__dirname, "..", "pkg", "package.json");

const cargo = fs.readFileSync(cargoPath, "utf8");
const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
if (!match) {
  console.error("ERROR: Could not find version in Cargo.toml");
  process.exit(1);
}

const cargoVersion = match[1];
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));

if (pkg.version === cargoVersion) {
  console.log(`pkg/package.json already at ${cargoVersion} — nothing to do.`);
  process.exit(0);
}

console.log(`Updating pkg/package.json: ${pkg.version} → ${cargoVersion}`);
pkg.version = cargoVersion;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
console.log("Done.");
