# Integration Guide: Sanctifier with Third-Party Audit Tools

This guide shows how to run **Sanctifier** alongside other security tools
(CodeQL, Semgrep, Slither) and consolidate their results into a single report,
GitHub Code Scanning view, or issue tracker (Jira / Linear).

Sanctifier emits **SARIF 2.1.0**, the OASIS-standard format that GitHub Code
Scanning and most aggregation tooling consume natively. Because every tool in
this guide can produce SARIF, the integrations all reduce to the same pattern:
*produce SARIF → optionally merge → upload / route*.

## Contents

- [How Sanctifier produces SARIF](#how-sanctifier-produces-sarif)
- [Merging multiple SARIF files](#merging-multiple-sarif-files)
- [GitHub Code Scanning: Sanctifier + CodeQL](#github-code-scanning-sanctifier--codeql)
- [Semgrep + Sanctifier](#semgrep--sanctifier)
- [Slither in a multi-tool SARIF pipeline](#slither-in-a-multi-tool-sarif-pipeline)
- [Exporting findings to Jira / Linear](#exporting-findings-to-jira--linear)
- [Sample workflow files](#sample-workflow-files)

---

## How Sanctifier produces SARIF

Sanctifier writes SARIF to **stdout**; redirect it to a file. The verified
command (see [`README.md`](../README.md) line 94 and
[`tooling/sanctifier-cli/src/commands/analyze.rs`](../tooling/sanctifier-cli/src/commands/analyze.rs)):

```bash
sanctifier analyze . --format sarif --min-severity high --exit-code > sanctifier-results.sarif
```

Flags used throughout this guide (all real — see `analyze.rs`):

| Flag | Meaning |
|------|---------|
| `--format text\|json\|ndjson\|sarif` | Output format. `sarif` emits SARIF 2.1.0. Default: `text`. |
| `--min-severity critical\|high\|medium\|low` | Threshold that `--exit-code` reacts to. Default: `high`. |
| `--exit-code` | Exit `1` when findings meet/exceed the threshold (so CI can fail). |
| `--profile strict\|lenient\|ci\|audit` | Preset that overrides `--exit-code`/`--min-severity`. |
| `--webhook-url <url>` | POST a scan-completed summary to a webhook (repeatable). |
| `--webhook-secret <secret>` | HMAC-SHA256 signing secret for `--webhook-url`. |

> Note: there is **no** `--output <file>` flag — Sanctifier writes to stdout, so
> use shell redirection (`> file.sarif`) as shown above and in the action.

The generated SARIF document has `tool.driver.name = "sanctifier"`,
`informationUri = https://github.com/HyperSafeD/Sanctifier`, and one
`runs[].results[]` entry per finding with `ruleId`, `level`
(`error`/`warning`/`note`), `message.text`, and a `physicalLocation` whose
`artifactLocation.uri` is the source file. See
[`docs/SARIF_METADATA.md`](./SARIF_METADATA.md) and
[`tooling/sanctifier-cli/src/commands/sarif.rs`](../tooling/sanctifier-cli/src/commands/sarif.rs)
for the exact shape.

### The GitHub Action

The repo ships a composite action ([`action.yml`](../action.yml)) that wraps the
CLI. Its inputs (use these exact names):

| Input | Default | Description |
|-------|---------|-------------|
| `path` | `.` | Contract path to analyze. |
| `version` | _(tag)_ | `sanctifier-cli` version to install. |
| `min-severity` | `high` | `critical\|high\|medium\|low\|info`. |
| `format` | `sarif` | `text\|json\|sarif`. |
| `upload-sarif` | `"true"` | Upload SARIF to Code Scanning when `format: sarif`. |
| `sarif-output` | `sanctifier-results.sarif` | Path for the generated SARIF file. |
| `use-docker` | `"false"` | Run via the `ghcr.io/hypersafed/sanctifier` image. |
| `debug` | `"false"` | Verbose action logging. |

Output: `findings-count`. When `upload-sarif: "true"`, the action uploads the
SARIF itself using `github/codeql-action/upload-sarif` — so you do **not** need a
separate upload step. (It skips upload on PRs from forks, where
`security-events: write` is unavailable.)

---

## Merging multiple SARIF files

There are two supported strategies. Prefer **(B)** for GitHub Code Scanning.

### A. Merge into one file with the SARIF multitool

[`@microsoft/sarif-multitool`](https://www.npmjs.com/package/@microsoft/sarif-multitool)
(the npm wrapper around the .NET SARIF SDK) merges multiple SARIF logs into a
single file — useful when a downstream system accepts only one upload, or for
archiving a combined audit artifact.

```bash
# One-off, no install:
npx @microsoft/sarif-multitool merge \
  sanctifier-results.sarif \
  semgrep.sarif \
  codeql.sarif \
  --recurse true \
  --output combined.sarif
```

Each input tool's run is preserved as a separate `runs[]` entry inside
`combined.sarif`, so provenance (which tool found what) is not lost.

### B. Upload each SARIF separately with a distinct `category`

GitHub Code Scanning **accepts multiple SARIF uploads per commit** and keeps
them separate as long as each upload uses a unique `category`. This is the
recommended approach: no merge step, and each tool's alerts are tracked,
de-duplicated, and resolved independently.

```yaml
- uses: github/codeql-action/upload-sarif@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
  with:
    sarif_file: sanctifier-results.sarif
    category: sanctifier        # <- unique per tool
- uses: github/codeql-action/upload-sarif@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
  with:
    sarif_file: semgrep.sarif
    category: semgrep           # <- different category
```

> If two uploads share a category, the later one **overwrites** the earlier
> one's alerts. Always give each tool its own category.

---

## GitHub Code Scanning: Sanctifier + CodeQL

A complete, working workflow lives at
[`.github/workflows/code-scanning-sanctifier.yml`](../.github/workflows/code-scanning-sanctifier.yml).
It runs Sanctifier and CodeQL as two parallel jobs and publishes both into Code
Scanning. The Sanctifier job uses the composite action (which uploads SARIF via
`github/codeql-action/upload-sarif` internally); the CodeQL job uses
`init` + `analyze`, where `analyze` uploads CodeQL's own SARIF with its own
category.

Key requirements:

```yaml
permissions:
  contents: read
  security-events: write   # required to upload SARIF
```

The Sanctifier portion, inline:

```yaml
- name: Run Sanctifier
  uses: HyperSafeD/Sanctifier@main
  continue-on-error: true
  with:
    path: .
    format: sarif
    min-severity: high
    upload-sarif: "true"
    sarif-output: sanctifier-results.sarif
```

If you would rather run the CLI directly and upload the SARIF yourself (for
extra flags, or to merge before upload), see
[`docs/integration-examples/sanctifier-cli-upload.yml`](./integration-examples/sanctifier-cli-upload.yml),
which uses `github/codeql-action/upload-sarif` with `category: sanctifier`.

### Running CodeQL alongside

The CodeQL job in the sample uses the standard pinned actions and a distinct
category, so its alerts coexist with Sanctifier's:

```yaml
- uses: github/codeql-action/init@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
  with:
    languages: javascript-typescript   # adjust to your codebase
- uses: github/codeql-action/analyze@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
  with:
    category: "/language:javascript-typescript"
```

> CodeQL does not currently analyze Soroban Rust contract logic the way
> Sanctifier does — that is exactly why you run them together. Point CodeQL at
> the languages it supports in your repo (e.g. the TypeScript dashboard under
> `frontend/`) and let Sanctifier cover the Soroban contracts.

---

## Semgrep + Sanctifier

Semgrep produces SARIF with `semgrep scan --sarif --output semgrep.sarif`.
Run it next to Sanctifier and upload each result set under its own category.

Full workflow:
[`docs/integration-examples/semgrep-sanctifier.yml`](./integration-examples/semgrep-sanctifier.yml).

Semgrep job, inline:

```yaml
semgrep:
  runs-on: ubuntu-latest
  container:
    image: semgrep/semgrep
  steps:
    - uses: actions/checkout@v6
    - run: semgrep scan --config p/rust --sarif --output semgrep.sarif
      env:
        SEMGREP_APP_TOKEN: ${{ secrets.SEMGREP_APP_TOKEN }}
    - uses: github/codeql-action/upload-sarif@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
      with:
        sarif_file: semgrep.sarif
        category: semgrep
```

`p/rust` is a reasonable default ruleset for Soroban's Rust codebase; swap in
your own Semgrep config or registry rulesets as needed. `SEMGREP_APP_TOKEN` is
optional and only needed for Semgrep Cloud Platform rulesets.

If you prefer **one** combined file over two uploads, merge first:

```bash
npx @microsoft/sarif-multitool merge semgrep.sarif sanctifier-results.sarif \
  --recurse true --output combined.sarif
# then a single upload of combined.sarif
```

---

## Slither in a multi-tool SARIF pipeline

[Slither](https://github.com/crytic/slither) is an **EVM / Solidity** static
analyzer. It does **not** analyze Soroban (Rust/WASM) contracts, so it does not
overlap with Sanctifier. It is relevant only in **multi-chain** repositories
that contain both Solidity (EVM) and Soroban (Stellar) contracts — there,
Slither covers the Solidity side and Sanctifier covers the Soroban side, and
both feed the same Code Scanning view.

Slither emits SARIF with the `--sarif` flag:

```bash
slither . --sarif slither.sarif
```

Then upload it like any other tool, under its own category:

```yaml
- name: Slither (Solidity / EVM contracts)
  run: slither ./evm-contracts --sarif slither.sarif || true
- uses: github/codeql-action/upload-sarif@dd903d2e4f5405488e5ef1422510ee31c8b32357 # v3
  with:
    sarif_file: slither.sarif
    category: slither
```

So a full multi-chain pipeline is just the same pattern repeated, one category
per tool: `sanctifier` (Soroban), `slither` (EVM), `semgrep`, `codeql`.

---

## Exporting findings to Jira / Linear

When you want tickets instead of (or in addition to) Code Scanning alerts,
parse the SARIF with `jq` and POST to the tracker's API. Sanctifier can also
fire a lightweight **scan-completed webhook** directly via
`--webhook-url`/`--webhook-secret` (HMAC-SHA256 signed), but that payload is a
*summary* (counts), not per-finding detail — for per-finding tickets, parse the
SARIF as below.

A ready-to-adapt workflow is at
[`docs/integration-examples/sarif-to-jira-linear.yml`](./integration-examples/sarif-to-jira-linear.yml).
Store credentials as repository secrets; never inline them.

### Extracting findings with jq

```bash
jq -c '.runs[].results[]
       | {ruleId,
          level,
          message: .message.text,
          uri: .locations[0].physicalLocation.artifactLocation.uri}' \
  sanctifier-results.sarif
```

### Jira (REST API v3 — create issue)

```bash
curl -sS -X POST "$JIRA_BASE_URL/rest/api/3/issue" \
  -u "$JIRA_EMAIL:$JIRA_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n \
        --arg proj "SEC" \
        --arg summary "[Sanctifier] $rule in $uri" \
        --arg body "$msg (file: $uri)" \
        '{fields: {project: {key: $proj},
                   issuetype: {name: "Bug"},
                   summary: $summary,
                   description: {type: "doc", version: 1,
                     content: [{type: "paragraph",
                       content: [{type: "text", text: $body}]}]}}}')"
```

### Linear (GraphQL — `issueCreate`)

```bash
curl -sS -X POST https://api.linear.app/graphql \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d "$(jq -n \
        --arg team "$LINEAR_TEAM_ID" \
        --arg title "[Sanctifier] $rule in $uri" \
        --arg desc "$msg (file: $uri)" \
        '{query: "mutation($t:String!,$d:String!,$team:String!){issueCreate(input:{title:$t,description:$d,teamId:$team}){success}}",
          variables: {t: $title, d: $desc, team: $team}}')"
```

> Tip: de-duplicate before creating tickets (e.g. search the tracker for an
> existing issue keyed on `ruleId + uri`) so re-runs don't open duplicates. The
> sample workflow leaves this to you to fit your team's conventions.

---

## Sample workflow files

| File | Integration |
|------|-------------|
| [`.github/workflows/code-scanning-sanctifier.yml`](../.github/workflows/code-scanning-sanctifier.yml) | **Tested, working** Code Scanning workflow: Sanctifier SARIF upload + CodeQL alongside. |
| [`docs/integration-examples/sanctifier-cli-upload.yml`](./integration-examples/sanctifier-cli-upload.yml) | Run the CLI directly and upload SARIF with `upload-sarif` + a category. |
| [`docs/integration-examples/semgrep-sanctifier.yml`](./integration-examples/semgrep-sanctifier.yml) | Semgrep + Sanctifier, each under its own category. |
| [`docs/integration-examples/sarif-to-jira-linear.yml`](./integration-examples/sarif-to-jira-linear.yml) | Parse SARIF with `jq` and open Jira / Linear tickets. |

All four YAML files are valid (parse cleanly with `yaml.safe_load`) and use the
same action versions as the rest of the repo (`actions/checkout@v6`,
`dtolnay/rust-toolchain@stable`, `actions/setup-python@v5`, and the
SHA-pinned `github/codeql-action/*@…v3`).
