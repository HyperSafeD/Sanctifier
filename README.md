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
*   `Sanctifier::guard_invariant(|ctx| ...)`: Enforce state invariants.
*   `Sanctifier::monitor_events()`: Ensure critical events are emitted.

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

## 🤝 Contributing
We welcome contributions from the Stellar community! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License
MIT
