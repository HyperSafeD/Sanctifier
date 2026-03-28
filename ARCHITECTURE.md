# Architecture & Components Overview

## 🎯 System Components Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SANCTIFIER DEPLOYMENT AUTOMATION                  │
│                    Runtime Guard Wrapper Platform                    │
└─────────────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────────────┐
│                          USER INTERFACES                              │
├───────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │   CLI Command    │  │  Bash Script     │  │ GitHub Actions   │  │
│  │                  │  │                  │  │                  │  │
│  │ sanctifier       │  │ ./deploy-        │  │ Workflow         │  │
│  │ deploy           │  │ soroban-         │  │ soroban-deploy   │  │
│  │ <PATH>           │  │ testnet.sh       │  │ .yml             │  │
│  │ --network        │  │ --network        │  │                  │  │
│  │ --validate       │  │ --validate       │  │ Schedules:       │  │
│  │                  │  │ --dry-run        │  │ • Push to main   │  │
│  │ Fast & Easy      │  │ --interval       │  │ • Every 6 hours  │  │
│  │                  │  │ --debug          │  │ • Manual trigger │  │
│  │ Single command   │  │                  │  │                  │  │
│  │ deployment       │  │ Production       │  │ CI/CD            │  │
│  │                  │  │ ready with       │  │ integration      │  │
│  │                  │  │ monitoring       │  │                  │  │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘  │
│           │                     │                     │             │
└───────────┼─────────────────────┼─────────────────────┼─────────────┘
            │                     │                     │
            └─────────────────────┼─────────────────────┘
                                  │
            ┌─────────────────────▼─────────────────────┐
            │    DEPLOYMENT ORCHESTRATION LAYER        │
            ├─────────────────────────────────────────┤
            │                                         │
            │  • Environment Validation              │
            │  • Contract Building (cargo WASM)      │
            │  • WASM Discovery & Verification       │
            │  • Soroban CLI Integration             │
            │  • Retry Logic (max 3 attempts)        │
            │  • Post-Deployment Validation          │
            │                                         │
            └─────────────────────┬─────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
    ┌────────────┐          ┌─────────────┐        ┌──────────────┐
    │  CONTRACT  │          │   LOGGING   │        │  VALIDATION  │
    │  BUILDING  │          │ & MANIFEST  │        │   CHECKING   │
    │            │          │             │        │              │
    │ • Compile  │          │ • Manifest  │        │ • Health     │
    │   Rust     │          │ • Call log  │        │   check()    │
    │ • WASM32   │          │ • Metrics   │        │ • get_stats()│
    │ • Optimize │          │ • Events    │        │ • Cycles:    │
    │            │          │             │        │   continuous │
    └──────┬─────┘          └──────┬──────┘        └──────┬──────┘
           │                       │                      │
           └───────────────────────┼──────────────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │   SOROBAN TESTNET            │
                    │                              │
                    │  • Contract Deployment       │
                    │  • Call Invocation           │
                    │  • Event Emission            │
                    │  • Storage Management        │
                    │                              │
                    └──────────────────────────────┘
```

## 📦 Core Components

### 1. Runtime Guard Wrapper Contract
```
RuntimeGuardWrapper {
  
  Public Functions:
  ├── init(wrapped_contract)
  ├── execute_guarded(fn_name, args) 
  ├── health_check() → bool
  ├── get_stats() → (u32, u32, u32)
  
  Internal Guards:
  ├── pre_execution_guards()
  │   └── Storage validation
  ├── post_execution_guards()
  │   └── Invariant verification
  └── Storage integrity checks
  
  Storage:
  ├── Instance
  │   ├── wrapped_contract_addr
  │   └── guard_config
  └── Persistent
      ├── call_log (Vec<Symbol>, max 100)
      ├── invariants_checked (u32)
      ├── guard_failures (Vec<Symbol>)
      └── exec_metrics (Vec<ExecutionMetrics>, max 1000)
}
```

### 2. Sanctifier CLI Deploy Command
```
Commands::Deploy(DeployArgs) {
  
  Input:
  ├── contract_path: PathBuf
  ├── network: String (testnet|futurenet|mainnet)
  ├── secret_key: Option<String>
  ├── account_id: Option<String>
  ├── validate: bool
  └── output_format: String
  
  Process:
  ├── validate_contract_path()
  ├── get_secret_key()
  ├── build_contract()
  ├── find_wasm_file()
  ├── deploy_contract()
  ├── validate_deployment() (if enabled)
  └── output_result()
  
  Output:
  ├── Text: Colored console output
  └── JSON: Machine-readable format
}
```

### 3. Bash Deployment Script
```
deploy-soroban-testnet.sh {
  
  Phases:
  ├── Phase 1: Environment Validation
  │   ├── Check tools (cargo, soroban, jq, curl)
  │   ├── Verify SOROBAN_SECRET_KEY
  │   ├── Validate network
  │   └── Check configuration
  │
  ├── Phase 2: Contract Discovery & Building
  │   ├── Find contracts in contracts/
  │   ├── Build each contract
  │   ├── Optimize WASM
  │   └── Verify artifacts
  │
  ├── Phase 3: Deployment
  │   ├── For each contract:
  │   │   ├── Deploy to network
  │   │   ├── Retry on failure (max 3)
  │   │   └── Record contract ID
  │   └── Update manifest
  │
  ├── Phase 4: Post-Deployment Validation
  │   ├── Call health_check()
  │   ├── Get stats
  │   └── Record validation result
  │
  └── Phase 5: Optional Continuous Validation
      ├── Loop every N seconds
      ├── Call health_check()
      ├── Update manifest
      └── Continue indefinitely
  
  Output:
  ├── .deployment-manifest.json (JSON)
  ├── .deployment.log (logs)
  └── Console (colored output)
}
```

### 4. GitHub Actions Workflow
```
Workflow: soroban-deploy.yml {
  
  Triggers:
  ├── on.push: branches: main
  │   └── paths: [contracts/runtime-guard-wrapper/**, scripts/**, .github/workflows/**]
  ├── on.schedule: "0 */6 * * *"          (Every 6 hours)
  └── on.workflow_dispatch                (Manual trigger)
  
  Job 1: build-and-deploy
  ├── Checkout code
  ├── Install Rust + targets
  ├── Cache dependencies
  ├── Format check
  ├── Clippy lint
  ├── Build WASM
  ├── Deploy to testnet
  ├── Run CLI test
  └── Upload artifacts
  
  Job 2: continuous-validation (needs: build-and-deploy)
  ├── Download manifest
  ├── Install Soroban CLI
  ├── For each deployed contract:
  │   ├── health_check()
  │   ├── get_stats()
  │   └── Record results
  └── Generate report
  
  Job 3: notification (needs: all)
  ├── Determine status
  ├── Create GitHub check
  ├── Post summary
  └── Add artifacts link
  
  Artifacts (30-day retention):
  ├── deployment-manifest-<RUN_ID>
  └── deployment-log-<RUN_ID>
}
```

## 🔄 Deployment Flow Diagram

```
START
  │
  ├─► Environment Validation
  │   ├─ Check tools exist
  │   ├─ Verify credentials
  │   └─ Validate config
  │
  ├─► Build Phase
  │   ├─ Compile to WASM
  │   ├─ Optimize
  │   └─ Verify artifact
  │
  ├─► Deploy Phase
  │   ├─ Deploy contract
  │   ├─ Retry on failure
  │   └─ Get contract ID
  │
  ├─► Validation Phase
  │   ├─ health_check()
  │   ├─ get_stats()
  │   └─ Record result
  │
  ├─► Manifest Update
  │   ├─ Add deployment record
  │   ├─ Record hash
  │   └─ Set status
  │
  ├─► Optional: Continuous Loop
  │   ├─ Sleep N seconds
  │   ├─ health_check()
  │   ├─ Update manifest
  │   └─ Repeat
  │
  └─► END
      Create logs & manifest
```

## 📊 Data Flow

```
User Input (CLI / Script / Actions)
      │
      ▼
┌─────────────────────┐
│ Configuration       │
│ .env.local or       │
│ GitHub Secrets      │
└────────┬────────────┘
         │
         ▼
┌─────────────────────────────────┐
│ Contract Source Code            │
│ contracts/runtime-guard-wrapper/ │
│ src/lib.rs                       │
└─────────┬───────────────────────┘
          │
          ▼
    ┌─────────────┐
    │ cargo build │─────────────┐
    └─────────────┘             │
                                ▼
                          ┌─────────────────┐
                          │ runtime_guard_  │
                          │ wrapper.wasm    │
                          └────────┬────────┘
                                   │
                                   ▼
                          ┌──────────────────┐
                          │ soroban contract │
                          │ deploy           │
                          └────────┬─────────┘
                                   │
                                   ▼
                          ┌──────────────────┐
                          │ Soroban Testnet  │
                          │ Contract ID: C.. │
                          └────────┬─────────┘
                                   │
          (Stored Records)         │
          ────────────────────────▼────────────────────────
          │                       │                       │
          ▼                       ▼                       ▼
    ┌──────────────┐      ┌────────────────┐    ┌─────────────┐
    │.deployment-  │      │health_check()  │    │get_stats()  │
    │manifest.json │      │valid? → bool   │    │→ (u32,...)  │
    └──────────────┘      └────────────────┘    └─────────────┘
         │
         ├─ Contract ID
         ├─ Deployment time
         ├─ WASM hash
         └─ Validation status

Continuous Loop (every N seconds):
    health_check() ──► Stored in manifest
```

## 🔐 Security & Secrets Flow

```
User Credentials
      │
      ├─► Local Development
      │   ├─ .env.local (git ignored)
      │   └─ source .env.local
      │
      └─► GitHub CI/CD
          ├─ Settings > Secrets
          └─ ${{ secrets.SOROBAN_SECRET_KEY }}
                      │
                      ▼
              ┌───────────────────┐
              │ GitHub Actions    │
              │ Container         │
              └────────┬──────────┘
                       │
              (masked in logs)
                       │
                       ▼
          soroban contract deploy \
          --source $SOROBAN_SECRET_KEY
```

## 🎯 State Management

```
Contract State (Soroban Testnet):
├── Instance Storage
│   ├─ wrapped_contract_addr: Address
│   └─ guard_config: (bool, bool, bool, bool)
│
└── Persistent Storage
    ├─ call_log: Vec<Symbol> (max 100)
    ├─ invariants_checked: u32
    ├─ guard_failures: Vec<Symbol>
    └─ exec_metrics: Vec<ExecutionMetrics> (max 1000)

Deployment State (Local File System):
├── .deployment-manifest.json
│   ├─ version: string
│   ├─ deployments: Array
│   │   ├─ contract_id: string
│   │   ├─ name: string
│   │   ├─ wasm_hash: string
│   │   ├─ deployed_at: ISO8601
│   │   ├─ last_validated: ISO8601
│   │   └─ status: enum
│   └─ last_updated: ISO8601
│
└── .deployment.log
    └─ Complete audit trail
```

## 📈 Metrics & Monitoring

```
Collected Metrics:
├─ Execution Count
│  └─ Total function calls tracked
│
├─ Invariant Checks
│  ├─ Pre-execution checks
│  ├─ Post-execution checks
│  └─ Total count
│
├─ Guard Failures
│  ├─ Failed validations
│  └─ Failure reasons
│
├─ Performance
│  ├─ Execution hash
│  ├─ Timestamp
│  ├─ Gas used
│  └─ Success/failure
│
└─ Health Status
   ├─ Storage accessible
   ├─ Metrics available
   └─ Overall health: bool
```

## 🚀 Deployment Lifecycle

```
Day 1: Initial Setup
└─ Deploy contract
   └─ Post-deployment validation passes

Day 1-N: Continuous Monitoring
└─ health_check() every 6 hours
   ├─ Contract state verified
   ├─ Metrics collected
   └─ Manifest updated

Day N+: Reporting
├─ Review .deployment-manifest.json
├─ Analyze .deployment.log
├─ Check GitHub Actions artifacts
└─ Generate compliance report
```

---

**This architecture provides:**
- ✅ Multiple entry points (CLI, Script, CI/CD)
- ✅ Comprehensive automation
- ✅ Continuous validation
- ✅ Complete audit trail
- ✅ Production-grade reliability
- ✅ Easy maintenance and extension

**Last Updated:** February 25, 2026
