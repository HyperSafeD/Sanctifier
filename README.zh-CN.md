<div align="center">

**[English](README.md)** | **[Español](README.es.md)** | **[中文](README.zh-CN.md)** | **[日本語](README.ja.md)** | **[Français](README.fr.md)**

</div>

<div align="center">
  <img src="branding/logo.png" width="220" alt="Sanctifier" />

  # Sanctifier

  ### 在别人利用漏洞之前，先发现它。

  **Stellar Soroban 智能合约安全副驾驶** — 静态分析、Z3 形式化验证、链上运行时保护，以及审计友好的仪表板，全部由一个 SARIF 清洁引擎驱动。

  [![CI](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml)
  [![Codecov](https://codecov.io/gh/HyperSafeD/Sanctifier/graph/badge.svg)](https://codecov.io/gh/HyperSafeD/Sanctifier)
  [![crates.io](https://img.shields.io/crates/v/sanctifier-cli.svg)](https://crates.io/crates/sanctifier-cli)
  [![Soroban Testnet](https://img.shields.io/badge/Soroban%20Testnet-Live-2dd4bf?style=flat-square&logo=stellar)](LIVE_TESTNET.md)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
</div>

---

## 为什么 Sanctifier 存在

当 EVM 合约发布漏洞时，社区拥有十年的工具 — Slither、Mythril、Foundry、Certora — 来发现它。Soroban 在 2024 年上线主网，但几乎没有这些基础设施。每个团队都从零开始编写相同的审查清单。每次审计都会重新发现相同的五个陷阱。

Sanctifier 是缺失的那一层。**一个引擎，十二条规范规则，三个部署表面。** 专为 Soroban 的授权模型、存储 TTL 语义、SEP-41 代币接口以及 gas/事件怪癖而构建。开源。审计级。即插即用于 CI。

---

## 它能发现什么

每个发现都有一个稳定的代码 — `S001..S012` — 这样你就可以在版本之间过滤、抑制和追踪它。

| 代码 | 发现内容 | 为什么危险 |
|------|-----------------|--------------|
| `S001` | 状态更改调用中缺少 `require_auth` | 任何人都可以耗尽你的合约 |
| `S002` | 合约路径中的 `panic!` / `unwrap` / `expect` | 状态锁定，无法恢复 |
| `S003` | 未检查的算术 — 溢出、下溢、截断 | 资金损失的静默舍入 |
| `S004` | 账本条目推大小阈值 | 写入时拒绝，交易中途 |
| `S005` | 数据路径之间的存储键冲突 | 跨功能数据损坏 |
| `S006` | 不安全模式 — 包括使用时间戳作为随机数 | 可预测的赢家，利用重放 |
| `S007` | 你的自定义 YAML 规则 | 强制执行你的代码风格 |
| `S008` | 不一致或缺失的事件发出 | 钱包和索引器失明 |
| `S009` | 未处理的 `Result` 返回值 | 静默失败伪装成成功 |
| `S010` | 升级 / 管理 / 治理风险 | 单密钥接管路径 |
| `S011` | Z3 反证的不变量 | 你没有的数学保证 |
| `S012` | SEP-41 代币接口偏差 | 钱包拒绝你的代币 |

此外，社区的**漏洞数据库**将已知的 CVE 风格模式（`SOL-2024-*`）与你的 AST 匹配 — 因此任何地方发布的漏洞都会到处成为发现。

---

## 现已在 Soroban 测试网上线 — 就在现在

这不是幻灯片。Sanctifier 的**运行时保护包装器**、**重入保护**和**易受攻击设计合约**已部署并发出链上审计事件，你今天就可以对其执行 `stellar contract invoke`。查看 **[LIVE_TESTNET.md](LIVE_TESTNET.md)** 获取地址、验证命令和事件日志。

```bash
# 在实时部署上跟踪实时保护事件
stellar events --network testnet --start-ledger <LATEST> \
  --id $RUNTIME_GUARD_CONTRACT_ID
```

---

## 五种使用方式

| 表面 | 用于 | 首次发现时间 |
|---|---|---|
| **`sanctifier` CLI** | 本地开发、脚本、热路径 | **30 秒** |
| **GitHub Action** | 每个 PR、每次推送 | **一次提交** |
| **Web 仪表板** (Next.js) | 审计员、审查员、黑客马拉松演示 | 拖放 `.rs` 文件 |
| **VS Code 扩展** | 输入时进行内联诊断 | 一次安装 |
| **链上运行时保护** | 部署后的取证跟踪 | 一次合约包装 |

所有这些都使用相同的引擎（它交叉编译到 WASM 用于浏览器路径），因此无论你在哪里扫描，发现都是逐位相同的。

---

## 30 秒快速入门

```bash
# 1. 安装
cargo install sanctifier-cli

# 2. 扫描
sanctifier analyze ./contracts/my-token

# 3. 为你的 README 生成徽章
sanctifier analyze . --format json > report.json
sanctifier badge --report report.json --svg-output sanctifier.svg
```

<details>
<summary><b>你将看到什么</b></summary>

```text
⚠️ 身份验证缺口
   → [S001] src/lib.rs:transfer — 缺少 require_auth
   → [S001] src/lib.rs:mint     — 缺少 require_auth

⚠️ 未检查的算术
   → [S003] src/lib.rs:transfer:30 — 运算符 `-`
   → [S003] src/lib.rs:transfer:33 — 运算符 `+`

⚠️ SEP-41 偏差
   → [S012] 缺少 `allowance` 函数

🛡️ 来自 DB v1.0.0 的 2 个已知漏洞匹配
   ❌ [SOL-2024-002] 代币传输缺少身份验证（严重）
   🔴 [SOL-2024-003] 未检查的余额下溢（高）

✨ 扫描完成 · 4 个发现 · exit 1
```

当存在严重/高发现时，退出代码为 `1` — 可以直接将其连接到 CI。

</details>

---

## 将其集成到你的仓库中（在一个 PR 中）

```yaml
# .github/workflows/sanctifier.yml
name: Sanctifier
on: [pull_request, push]
permissions: { contents: read, security-events: write }
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: HyperSafeD/Sanctifier@main
        with:
          path: .
          format: sarif
          min-severity: high
          upload-sarif: "true"
```

SARIF 会进入 GitHub 代码扫描，以便审查者在 PR 上看到内联注释。

---

## 在本地运行仪表板

```bash
cd frontend
npm install
npm run dev
# → http://localhost:3000
```

- **`/scan`** — 拖入 `.rs` 文件，在 <2 秒内获得发现
- **`/dashboard`** — 加载 JSON 报告，按严重程度深入查看，查看实时调用图
- **`/playground`** — 尝试罐装易受攻击合约（身份验证缺口、溢出、不安全的 PRNG，…）
- **`/terminal`** — 终端模拟器中的 `sanctifier`，用于引导式演示

---

## 安装选项

| 方法 | 命令 |
|--------|---------|
| **crates.io** | `cargo install sanctifier-cli` |
| **从源代码** | `git clone https://github.com/HyperSafeD/Sanctifier && cd Sanctifier && make release` |
| **Codespaces** | [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/HyperSafeD/Sanctifier) |
| **Docker** | `docker run --rm -v $PWD:/src ghcr.io/hypersafed/sanctifier analyze /src` |

**先决条件：** Rust 1.78+，以及用于 Z3 形式化验证后端的 `libz3-dev` 和 `clang`/`libclang-dev`。

```bash
# Debian/Ubuntu
sudo apt-get install libz3-dev clang libclang-dev
# macOS
brew install z3 llvm
```

使用 `cargo install sanctifier-cli --no-default-features` 完全跳过 Z3 — 除 `S011` 外的每条规则仍会运行。

---

## CLI 参考

```bash
sanctifier analyze    [PATH] [--format text|json] [--limit BYTES] [--webhook-url URL]...
sanctifier diff       [PATH] --baseline <report.json>
sanctifier watch      [PATH]              # 文件更改时重新运行
sanctifier workspace  [PATH]              # cargo-workspace 感知扫描
sanctifier callgraph  [PATH] --output callgraph.dot
sanctifier badge      --report report.json --svg-output sanctifier.svg
sanctifier fix        [PATH] --rule S003  # 应用修补程序修复
sanctifier verify     [PATH]              # 仅 Z3 不变量传递
sanctifier deploy     ...                 # 部署运行时保护
sanctifier doctor                         # 环境诊断
sanctifier init                           # 生成 .sanctify.toml
sanctifier update                         # 使用校验和检查自我更新
```

每个子命令都支持 `--format json` 以供机器使用。

---

## 输出是契约，不是氛围

`--format json` 输出根据 [`schemas/analysis-output.json`](schemas/analysis-output.json)（JSON Schema draft-07）进行验证。每个报告都携带一个独立于 CLI 版本递增的 `schema_version`，因此下游工具可以固定到架构而无需耦合到发布节奏。

```jsonc
{
  "metadata":       { "version": "0.1.0", "format": "sanctifier-ci-v1", "timestamp": "…" },
  "summary":        { "critical": 0, "high": 0, "medium": 2, "low": 0 },
  "findings":       { "auth_gaps": [...], "arithmetic_issues": [...], "storage_collisions": [...] },
  "vuln_db_matches": [{ "id": "SOL-2024-002", "severity": "CRITICAL", "matched_at": "…" }],
  "schema_version": "1.0.0"
}
```

SARIF 2.1.0 输出是 GitHub 代码扫描和任何 SAST 聚合器的规范。

---

## 配置 — `.sanctify.toml`

```toml
ignore_paths        = ["target", ".git"]
enabled_rules       = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit        = 64000
approaching_threshold = 0.8
strict_mode         = false

[[custom_rules]]
name     = "no_unsafe_block"
pattern  = 'unsafe\s*\{'
severity = "error"
```

自定义规则支持完整的 YAML DSL — 请参阅 [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md)。

---

## 路线图

Sanctifier 分波次发布。已完成的内容、下一步内容、愿望清单：

**已发布**
- 12 条规范分析规则（S001–S012）具有稳定代码
- CLI、GitHub Action、Web 仪表板、VS Code 扩展、WASM 构建
- 实时测试网运行时保护合约发出链上审计事件
- SARIF + JSON 输出、draft-07 架构、徽章生成器
- 差异模式、监视模式、cargo-workspace 扫描、修补程序

**进行中**（请参阅 [contrib-wave issues](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle)）
- `/api/ai/explain` 的真实 LLM 提供商（当前为存根）
- 编辑器无关的 `sanctifier lsp`，用于 Neovim / Helix / Zed
- 流式 `--ndjson` 输出用于增量管道
- GitHub PR 注释格式化程序，带有与基线的增量
- 20 多条新引擎规则（授权竞争、TTL 增加、跨合约 `try_call`、通过解构的污点分析，…）

**愿望清单**
- 托管 REST API、Stellar Laboratory 插件、cargo-sanctify 子命令 shim、用于记录的运行时调用的异常检测规则引擎

---

## 项目布局

```text
Sanctifier/
├── tooling/
│   ├── sanctifier-cli/        # CLI 二进制文件（你安装的那个）
│   ├── sanctifier-core/       # 静态分析引擎 + Z3 后端
│   └── sanctifier-wasm/       # 引擎的浏览器/Node WASM 构建
├── frontend/                  # Next.js 仪表板、游乐场、终端
├── vscode-extension/          # VS Code 诊断集成
├── contracts/                 # Soroban 合约（fixtures + 实时目标）
│   ├── runtime-guard-wrapper/ # ← 部署到测试网
│   ├── reentrancy-guard/      # ← 部署到测试网
│   └── vulnerable-contract/   # ← 部署到测试网（演示目标）
├── schemas/
│   └── analysis-output.json   # JSON Schema (draft-07) — 在 CI 中验证
├── data/
│   └── vulnerability-db.json  # 社区提供的 CVE 风格模式
├── action.yml                 # GitHub 复合操作
├── benchmarks/                # 性能语料库
├── specs/                     # OpenAPI + RFC 草案
└── docs/                      # 指南、ADR、威胁模型、案例研究
```

---

## 文档

| 如果你想… | 阅读 |
|-----------------|------|
| 在 10 分钟内入门 | [docs/getting-started.md](docs/getting-started.md) |
| 了解每个错误代码 | [docs/error-codes.md](docs/error-codes.md) |
| 将运行时保护连接到你的合约 | [docs/runtime-guards-integration.md](docs/runtime-guards-integration.md) |
| 设置 CI | [docs/ci-cd-setup.md](docs/ci-cd-setup.md) |
| 部署到测试网 | [docs/soroban-deployment.md](docs/soroban-deployment.md) |
| 编写你自己的规则 | [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md) |
| 查看基准测试 | [docs/case-studies/soroban-examples.md](docs/case-studies/soroban-examples.md) |
| 审查威胁模型 | [docs/security-threat-model.md](docs/security-threat-model.md) |
| 浏览设计决策 | [docs/adr/](docs/adr/) |

---

## 贡献

我们正在获得动力，我们需要帮助。**~100 个手动策划的 [`[contrib-wave]`](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle) 问题**已上线，每个问题都有问题陈述、验收标准、文件指针和难度提示。每个技能级别都有一个 `good first issue` — bash、Rust、TypeScript、Next.js、GitHub Actions、文档编写、合约编写。

从 [CONTRIBUTING.md](CONTRIBUTING.md) 开始，然后选择一个问题并打个招呼。

---

## 许可证

MIT — 请参阅 [LICENSE](LICENSE)。

<div align="center">
  <sub>为 Stellar Soroban 生态系统构建 · 主网不原谅 · 审计级，在 CI 中。</sub>
</div>
