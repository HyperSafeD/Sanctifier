# Sanctifier 🛡️

<p align="center">
  <img src="branding/logo.png" width="300" alt="Sanctifier Logo">
</p>

**Sanctifier** is a comprehensive security and formal verification suite built specifically for [Stellar Soroban](https://soroban.stellar.org/) smart contracts. In the high-stakes environment of DeFi and decentralized applications, "code is law" only holds true if the code is secure. Sanctifier ensures your contracts are not just compiled, but *sanctified*—rigorously tested, formally verified, and runtime-guarded against vulnerabilities.

## 📂 Project Structure

```text
Sanctifier/
├── contracts/          # Soroban smart contracts (examples & templates)
├── frontend/           # Next.js Web Interface for the suite
├── tooling/            # The core Rust analysis tools
│   ├── sanctifier-cli  # CLI tool for developers
│   └── sanctifier-core # Static analysis logic
├── scripts/            # Deployment and CI scripts
└── docs/               # Documentation
```

## 🚀 Key Features

### 1. Static Sanctification (Static Analysis)
Sanctifier scans your Rust/Soroban code before deployment to detect:
*   **Authorization Gaps**: ensuring `require_auth` is present in all privileged functions.
*   **Storage Collisions**: analyzing `Instance`, `Persistent`, and `Temporary` storage keys.
*   **Resource Exhaustion**: estimating instruction counts to prevent OOG.

### 2. Runtime Guardians
A library of hook-based guards that you can integrate into your contracts:
*   Runtime invariant checks via `SanctifiedGuard`.
*   Step-by-step integration guide: [`docs/runtime-guards-integration.md`](docs/runtime-guards-integration.md)

### 3. Automated Deployment & Validation (NEW!)
Deploy runtime guard wrapper contracts to Soroban testnet with continuous validation:
*   **CLI Deployment**: One-command contract deployment with `sanctifier deploy`
*   **Bash Automation**: Production-ready scripts for testnet deployment
*   **CI/CD Integration**: GitHub Actions workflow for automated deployment and monitoring
*   **Continuous Validation**: Periodic health checks and execution metrics collection

## 📦 Installation (CLI)

```bash
cargo install --path tooling/sanctifier-cli
```

## 🛠 Usage

### Analyze a Project
Run the analysis suite on your Soroban project:

```bash
sanctifier analyze ./contracts/my-token
```

### Notify Webhooks on Scan Completion
Send scan completion notifications to one or more webhook endpoints:

```bash
sanctifier analyze ./contracts/my-token --webhook-url https://hooks.slack.com/services/XXX/YYY/ZZZ --webhook-url https://discord.com/api/webhooks/ID/TOKEN
```

### Update Sanctifier
Check for and download the latest Sanctifier binary:

```bash
sanctifier update
```

### Generate a README Security Badge
Create an SVG badge and markdown snippet from a JSON scan report:

```bash
sanctifier analyze . --format json > sanctifier-report.json
sanctifier badge --report sanctifier-report.json --svg-output badges/sanctifier-security.svg --markdown-output badges/sanctifier-security.md
```

## 🤝 Contributing
We welcome contributions from the Stellar community! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 🔎 Finding Codes
Unified finding codes (`S001`...`S007`) are documented in [docs/error-codes.md](docs/error-codes.md).

## 📄 License
MIT
