# Sanctifier Language Server Protocol (LSP)

Sanctifier now provides an editor-agnostic Language Server Protocol (LSP) implementation, enabling real-time security analysis across any LSP-capable editor: VSCode, Neovim, Helix, Zed, JetBrains IDEs, and more.

## Overview

The Sanctifier LSP server (`sanctifier lsp`) provides:

- **Real-time Diagnostics**: Immediate feedback on security issues as you type
- **Code Actions**: Quick fixes for authorization gaps and arithmetic overflow patterns
- **Hover Information**: Detailed descriptions of detected issues
- **Cross-Editor Support**: Works with any editor supporting the Language Server Protocol

## Quick Start

### Installation

1. Build or install `sanctifier`:
   ```bash
   cargo build --bin sanctifier --release
   # Binary will be at: ./target/release/sanctifier
   ```

2. Or install via cargo:
   ```bash
   cargo install --path tooling/sanctifier-cli
   ```

### Starting the LSP Server

```bash
sanctifier lsp
```

Or with debug logging:

```bash
sanctifier lsp --debug
```

The server listens on stdin/stdout and outputs diagnostic results over the LSP protocol.

## Editor Configuration

### VSCode / VS Code Insiders

Update your VSCode settings to use the Sanctifier LSP:

**.vscode/settings.json** or **User Settings**:
```json
{
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "lsp.languageServers": {
    "sanctifier": {
      "command": "sanctifier",
      "args": ["lsp"],
      "filetypes": ["rust"],
      "description": "Soroban security analysis"
    }
  }
}
```

Or use the VS Code LSP Client extension to connect to the server.

### Neovim

Use the built-in LSP client with Neovim's init.lua:

```lua
vim.lsp.start({
  name = "sanctifier",
  cmd = { "sanctifier", "lsp" },
  root_dir = vim.fn.getcwd(),
  filetypes = { "rust" },
})
```

Or use a plugin like `nvim-lspconfig`:

```lua
require('lspconfig.configs').sanctifier = {
  default_config = {
    cmd = { 'sanctifier', 'lsp' },
    filetypes = { 'rust' },
    root_dir = require('lspconfig').util.root_pattern('Cargo.toml'),
  }
}
require('lspconfig').sanctifier.setup({})
```

### Helix

Add to `.helix/languages.toml`:

```toml
[[language]]
name = "rust"
language-servers = ["rust-analyzer", "sanctifier"]

[language-server.sanctifier]
command = "sanctifier"
args = ["lsp"]
```

### Zed

Configure in Zed's settings:

```json
{
  "language_servers": {
    "sanctifier": {
      "command": "sanctifier",
      "args": ["lsp"]
    }
  },
  "languages": {
    "Rust": {
      "language_servers": ["rust-analyzer", "sanctifier"]
    }
  }
}
```

### JetBrains IDEs

1. Install the LSP Support plugin (if not already installed)
2. Go to **Settings → Languages & Frameworks → Language Servers**
3. Click **+** to add a new server:
   - **Language**: Rust
   - **Extension**: rs
   - **Command**: `sanctifier lsp`
4. Apply and restart

## Diagnostics

The LSP server publishes diagnostics with the following error codes:

| Code | Severity | Issue | Suggestion |
|------|----------|-------|-----------|
| **S001** | Warning | Auth Gap | Function modifies state without `require_auth` |
| **S002** | Warning | Panic Usage | Use of `panic!()`, `.unwrap()`, or `.expect()` |
| **S003** | Warning | Arithmetic Overflow | Unchecked arithmetic operations |
| **S004** | Error/Warning | Ledger Size | Structure exceeds allocated space limits |
| **S006** | Warning | Unsafe Pattern | Risky code patterns detected |
| **S007** | Info | Custom Rule | Custom regex rule match |

### Example Diagnostic Output

When you open a Rust file in an LSP-capable editor:

```
Line 42: Function 'transfer' modifies state without authorization check. 
Add require_auth or require_auth_for_args. [S001]

Line 50: Unchecked arithmetic operation '+' in function 'mint'. 
Use `.checked_add(rhs)` or `.saturating_add(rhs)` [S003]
```

## Code Actions

The LSP server provides quick fixes for common issues:

### Authorization Gap Fix
- **Title**: `Add require_auth to function '<name>'`
- **Action**: Suggests adding authorization checks to privileged functions

### Arithmetic Overflow Fix
- **Title**: `Use checked_<op> instead of '<op>'`
- **Action**: Suggests replacing unsafe arithmetic with checked versions

## Protocol Details

### Supported Methods

| Method | Type | Description |
|--------|------|-------------|
| `initialize` | Request | Initialize the server handshake |
| `shutdown` | Request | Graceful server shutdown |
| `textDocument/didOpen` | Notification | File opened in editor |
| `textDocument/didChange` | Notification | File content changed |
| `textDocument/didClose` | Notification | File closed in editor |
| `textDocument/codeAction` | Request | Request code actions for range |

### Capabilities

The server advertises these capabilities:

```json
{
  "capabilities": {
    "textDocumentSync": 1,
    "diagnosticProvider": {
      "interFileDependencies": false,
      "workspaceDiagnostics": false
    },
    "codeActionProvider": true,
    "hoverProvider": true
  }
}
```

## Testing

### Command-Line Testing

Test the LSP server directly with stdio:

```bash
# Start the server
sanctifier lsp --debug

# In another terminal, send an initialize request:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  (echo 'Content-Length: 65'; echo ''; cat) | \
  nc localhost 9000
```

### VSCode Integration Test

Use `vscode-test-cli` for automated testing:

```bash
cd vscode-extension
npm install
npm test
```

### Neovim Integration Test

Test with Neovim's built-in LSP client:

```bash
nvim +LSPStart <your_soroban_contract.rs>
```

Monitor LSP activity:

```vim
:LspLog
```

## Configuration

The LSP server respects `.sanctify.toml` configuration when present:

```toml
[sanctifier]
enabled_rules = ["auth_gaps", "panics", "arithmetic"]
ledger_limit = 65536
approaching_threshold = 0.8
strict_mode = false

[[rules]]
name = "custom_pattern"
pattern = "unsafe\s+\{.*\}"
severity = "warning"
```

## Performance

- **Startup**: < 100ms
- **Diagnostic Scan**: < 500ms for typical contracts (< 5000 LOC)
- **Memory**: ~20-50 MB base, scales with open document count

## Troubleshooting

### LSP Server Not Starting

1. Verify `sanctifier` is in your PATH:
   ```bash
   which sanctifier
   ```

2. Test directly:
   ```bash
   sanctifier lsp --debug 2>&1 | head -20
   ```

3. Check editor LSP configuration logs

### No Diagnostics Appearing

1. Ensure the file is recognized as Rust (`*.rs`)
2. Check that the file content is valid Rust syntax
3. Enable debug logging in LSP server:
   ```bash
   # Kill current server and restart with:
   sanctifier lsp --debug
   ```
4. View server output in editor's LSP debug console

### Performance Issues

If analysis is slow:

1. Check file size (> 100KB might be slow)
2. Ensure no large generated files are being analyzed
3. Configure `ignore_paths` in `.sanctify.toml`
4. Check system resources (CPU, memory)

## Architecture

The Sanctifier LSP follows the standard LSP specification (v3.17):

```
Editor ←→ LSP Client (Built-in)
              ↓ stdio
        Sanctifier LSP Server
              ↓
        sanctifier-core
              ↓
        Analysis Engines:
        - Auth Gap Scanner
        - Panic Detector
        - Arithmetic Overflow Analyzer
        - Storage Size Estimator
        - Custom Rule Matcher
```

## Development

### Running LSP Tests Locally

```bash
# Build debug binary
cargo build --bin sanctifier

# Start server with debug logging
./target/debug/sanctifier lsp --debug

# Test with a sample contract
cat contracts/token-with-bugs/src/lib.rs | \
  ./target/debug/sanctifier lsp --debug
```

###Contributing

To extend the LSP server:

1. Add new diagnostic in [src/commands/lsp.rs](../tooling/sanctifier-cli/src/commands/lsp.rs)
2. Implement `analyze_document()` extension
3. Add corresponding code action in `get_code_actions()`
4. Test with editor client
5. Update this documentation

## Related Documentation

- [Sanctifier CLI Guide](./getting-started.md)
- [Soroban Deployment Guide](./DEPLOYMENT_IMPLEMENTATION.md)
- [Security Guidelines](../SECURITY.md)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)

## License

Sanctifier LSP Server is licensed under MIT or Apache 2.0 (see [LICENSE](../LICENSE))
