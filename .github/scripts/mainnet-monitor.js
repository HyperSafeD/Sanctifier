const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const workspace = process.env.GITHUB_WORKSPACE || process.cwd();
const liveMainnetPath = path.join(workspace, "LIVE_MAINNET.md");
const manifestPath = path.join(workspace, ".deployment-manifest.json");
const monitorDir = path.join(workspace, "monitor");
const network = "mainnet";
const identity = "mainnet-monitor";

// ── Entry-point probes ─────────────────────────────────────────────────────────
// The Stellar CLI builds `invoke -- <fn>` subcommands from the contract's own
// spec (fetched from RPC). A function that is not exported fails instantly with
// "unrecognized subcommand", which lets us test for capability safely and
// read-only (no source account funding, no transaction submission).

const PROBES_BY_CATEGORY = {
  token: [{ fn: "total_supply", kind: "supply" }],
  guard: [
    { fn: "is_paused", kind: "pause" },
    { fn: "health_check", kind: "health" },
    { fn: "get_stats", kind: "guard_stats" },
  ],
  flashloan: [
    { fn: "is_paused", kind: "pause" },
    { fn: "health_check", kind: "health" },
    { fn: "stats", kind: "flash_stats" },
  ],
  default: [
    { fn: "is_paused", kind: "pause" },
    { fn: "health_check", kind: "health" },
    { fn: "admin", kind: "admin" },
    { fn: "get_admin", kind: "admin" },
  ],
};

// ── Deployment set resolution ────────────────────────────────────────────────

function parseLiveMainnet(markdown) {
  return markdown
    .split(/\r?\n/)
    .map((line) => {
      const match = line.match(
        /^\|\s*\*\*(?<name>[^*]+)\*\*(?<suffix>[^|]*)\|\s*`(?<id>C[A-Z0-9]+)`\s*\|/,
      );
      if (!match?.groups) {
        return null;
      }
      const suffix = match.groups.suffix.replace(/\s+/g, " ").trim();
      return {
        name: `${match.groups.name}${suffix ? ` ${suffix}` : ""}`.trim(),
        id: match.groups.id,
        category: inferCategory(match.groups.name.toLowerCase()),
        expected_admin: undefined,
      };
    })
    .filter(Boolean);
}

function inferCategory(name) {
  if (/guard|wrapper/i.test(name)) return "guard";
  if (/flashloan|breaker|pause/i.test(name)) return "flashloan";
  return "default";
}

function resolveContracts() {
  // 1. Explicit override (workflow_dispatch input / env).
  const override = process.env.MAINNET_MONITOR_CONTRACTS;
  if (override) {
    const parsed = JSON.parse(override);
    if (!Array.isArray(parsed)) {
      throw new Error("MAINNET_MONITOR_CONTRACTS must be a JSON array");
    }
    return parsed.map((entry) => ({
      name: entry.name || entry.id,
      id: entry.id,
      category: entry.category || "default",
      expected_admin: entry.expected_admin,
    }));
  }

  // 2. Deployment manifest produced by scripts/deploy-soroban-testnet.sh.
  if (fs.existsSync(manifestPath)) {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    const deployments = manifest.deployments || [];
    return deployments
      .filter((d) => !d.network || d.network === "mainnet")
      .map((d) => ({
        name: d.name,
        id: d.contract_id,
        category: inferCategory(String(d.name || "")),
        expected_admin: undefined,
      }));
  }

  // 3. LIVE_MAINNET.md table (mirrors the testnet monitor convention).
  if (fs.existsSync(liveMainnetPath)) {
    return parseLiveMainnet(fs.readFileSync(liveMainnetPath, "utf8"));
  }

  // No mainnet deployments configured yet — scheduled runs report a clean,
  // informational status instead of failing.
  return [];
}

// ── Probes ────────────────────────────────────────────────────────────────────

function runCmd(args) {
  return spawnSync("stellar", args, { encoding: "utf8", timeout: 30_000 });
}

const READONLY_HINT = /Simulation identified as read-only/i;

function stripReadOnlyHint(output) {
  return output
    .split(/\r?\n/)
    .filter((l) => !READONLY_HINT.test(l))
    .join("\n")
    .trim();
}

function probeValue(probe) {
  const result = runCmd([
    "contract",
    "invoke",
    "--id",
    probe.id,
    "--source",
    identity,
    "--network",
    network,
    "--",
    probe.fn,
  ]);

  const stdout = stripReadOnlyHint(result.stdout);
  const stderr = result.stderr.trim();
  const output = [stdout, stderr].filter(Boolean).join("\n");

  if (result.status === 0) {
    return { status: "ok", value: stdout, output, exported: true };
  }

  if (/unrecognized subcommand/i.test(stderr)) {
    return { status: "missing", value: undefined, output, exported: false };
  }

  if (/Missing required argument|suggestions:/i.test(stderr)) {
    return { status: "needs-args", value: undefined, output, exported: true };
  }

  return { status: "failed", value: undefined, output, exported: true };
}

function probeAvailability(contract) {
  const result = runCmd([
    "contract",
    "fetch",
    "--id",
    contract.id,
    "--network",
    network,
  ]);
  const stdout = result.stdout.trim();
  const stderr = result.stderr.trim();
  const output = [stdout, stderr].filter(Boolean).join("\n");
  // Network built-in asset contracts (e.g. the native XLM SAC) have no
  // downloadable code binary but are fully live — treat them as reachable.
  const builtin = /network built-in asset contract/i.test(stderr);
  return {
    healthy: result.status === 0 || builtin,
    exitCode: result.status,
    output,
  };
}

function boolValue(text) {
  if (/^"?true"?$/i.test(text.trim())) return true;
  if (/^"?false"?$/i.test(text.trim())) return false;
  return null;
}

function evaluateProbe(probe, result) {
  switch (probe.kind) {
    case "pause":
      if (result.status === "ok") {
        const paused = boolValue(result.value);
        if (paused === true) {
          return { ...probe, ...result, passed: false, critical: true };
        }
        if (paused === false) {
          return { ...probe, ...result, passed: true };
        }
      }
      return { ...probe, ...result, passed: result.status !== "failed" };
    case "health":
      if (result.status === "ok") {
        const healthy = boolValue(result.value);
        if (healthy === false) {
          return { ...probe, ...result, passed: false };
        }
        if (healthy === true) {
          return { ...probe, ...result, passed: true };
        }
      }
      return { ...probe, ...result, passed: result.status !== "failed" };
    case "guard_stats": {
      if (result.status !== "ok") {
        return { ...probe, ...result, passed: result.status !== "failed" };
      }
      const match = result.value.match(/[-0-9]+/g);
      const stats = match ? match.map(Number) : [];
      const failures = stats[2] ?? 0;
      return { ...probe, ...result, passed: failures === 0, stats };
    }
    case "flash_stats": {
      if (result.status !== "ok") {
        return { ...probe, ...result, passed: result.status !== "failed" };
      }
      const match = result.value.match(/[-0-9]+/g);
      const stats = match ? match.map(Number) : [];
      return { ...probe, ...result, passed: true, stats };
    }
    case "supply":
      if (result.status === "ok") {
        const supply = Number(result.value.replace(/[^0-9-]/g, ""));
        if (Number.isFinite(supply)) {
          return { ...probe, ...result, passed: supply > 0, supply };
        }
      }
      return { ...probe, ...result, passed: result.status !== "failed" };
    case "admin":
      if (result.status === "ok") {
        return { ...probe, ...result, passed: true, admin: result.value };
      }
      return { ...probe, ...result, passed: result.status !== "failed" };
    default:
      return { ...probe, ...result, passed: result.status !== "failed" };
  }
}

// ── Reporting ────────────────────────────────────────────────────────────────

function writeGitHubOutput(results, healthy, pausedCount) {
  const githubOutput = process.env.GITHUB_OUTPUT;
  if (!githubOutput) {
    return;
  }
  const failures = results.filter(
    (r) => r.health === "unhealthy" || r.paused === true,
  );
  const degraded = results.filter((r) => r.health === "degraded").length;
  fs.appendFileSync(githubOutput, `healthy=${healthy}\n`);
  fs.appendFileSync(githubOutput, `failures=${failures.length}\n`);
  fs.appendFileSync(githubOutput, `degraded=${degraded}\n`);
  fs.appendFileSync(githubOutput, `total=${results.length}\n`);
  fs.appendFileSync(githubOutput, `paused=${pausedCount > 0}\n`);
}

function summaryRows(results) {
  return results.map((result) => {
    const paused = result.probeEvals.some(
      (p) => p.kind === "pause" && p.passed === false && p.critical,
    )
      ? "paused"
      : "active";
    const health = result.health;
    const admin =
      result.probeEvals.find((p) => p.kind === "admin" && p.admin)?.admin ||
      "n/a";
    const stats =
      result.probeEvals.find((p) => p.stats)?.stats?.join("/") || "n/a";
    return `| ${result.name} | \`${result.id}\` | ${health} | ${paused} | ${admin} | ${stats} |`;
  });
}

function writeSummary(results, healthy, message, monitored) {
  const githubStepSummary = process.env.GITHUB_STEP_SUMMARY;
  if (!githubStepSummary) {
    return;
  }

  const lines = [
    "## Mainnet health monitor",
    "",
    typeof message === "string" ? message : "",
    "",
    `Badge: ${monitored ? (healthy ? `${results.length}/${results.length} healthy` : `${results.length - results.filter((r) => r.health !== "healthy").length}/${results.length} healthy`) : "no deployments configured"}`,
    "",
    "| Contract | Address | Health | Pause | Admin | Stats |",
    "|---|---|---|---|---|---|",
    ...summaryRows(results),
    "",
  ];

  fs.appendFileSync(githubStepSummary, lines.join("\n"));
}

// ── Main ─────────────────────────────────────────────────────────────────────

if (require.main === module) {
  const contracts = resolveContracts();

  fs.mkdirSync(monitorDir, { recursive: true });

  let results;
  let healthy;
  let message;

  if (contracts.length === 0) {
    results = [];
    healthy = true;
    message =
      "No mainnet deployments are configured yet — add `LIVE_MAINNET.md`, a `.deployment-manifest.json`, or pass `MAINNET_MONITOR_CONTRACTS`.";
  } else {
    results = contracts.map((contract) => {
      const checkedAt = new Date().toISOString();
      const availability = probeAvailability(contract);

      const probes = PROBES_BY_CATEGORY[contract.category] || PROBES_BY_CATEGORY.default;
      const withId = probes.map((p) => ({ ...p, id: contract.id }));
      const probeEvals = withId.map((probe) =>
        evaluateProbe(probe, probeValue(probe)),
      );

      const paused = probeEvals.some(
        (p) => p.kind === "pause" && p.critical,
      );
      const failedProbes = probeEvals.filter((p) => p.status === "failed");
      const unmet = probeEvals.filter((p) => !p.passed);

      let health = "healthy";
      if (!availability.healthy || paused || unmet.some((p) => p.critical)) {
        health = "unhealthy";
      } else if (unmet.length > 0 || failedProbes.length > 0) {
        health = "degraded";
      }

      return {
        ...contract,
        checkedAt,
        health,
        paused,
        availability,
        probeEvals,
        output: [
          availability.healthy
            ? ""
            : `availability: ${availability.output}`,
          ...failedProbes.map(
            (p) => `probe ${p.fn}: ${p.output.slice(0, 200)}`,
          ),
        ]
          .filter(Boolean)
          .join("\n"),
      };
    });

    const unhealthy = results.filter(
      (r) => r.health === "unhealthy" || r.paused,
    );
    healthy = unhealthy.length === 0;
    message = healthy
      ? "All configured mainnet contracts are healthy."
      : `${unhealthy.length} contract(s) failed their scheduled mainnet health check.`;
  }

  const pausedResults = results.filter((r) => r.paused);
  const failures = results.filter(
    (r) => r.health === "unhealthy" || r.paused,
  );

  const badge = {
    schemaVersion: 1,
    label: "mainnet monitor",
    message:
      pausedResults.length > 0
        ? `paused: ${pausedResults.length} contract(s)`
        : healthy
          ? `${contracts.length}/${contracts.length} healthy`
          : `${contracts.length - failures.length}/${contracts.length} healthy`,
    color:
      pausedResults.length > 0 ? "red" : healthy ? "brightgreen" : "orange",
  };

  fs.writeFileSync(
    path.join(monitorDir, "results.json"),
    JSON.stringify(
      { checkedAt: new Date().toISOString(), network, contracts: results },
      null,
      2,
    ),
  );
  fs.writeFileSync(
    path.join(monitorDir, "failures.json"),
    JSON.stringify(failures, null, 2),
  );
  fs.writeFileSync(
    path.join(monitorDir, "status.json"),
    JSON.stringify(badge, null, 2),
  );

  writeGitHubOutput(results, healthy, pausedResults.length);
  writeSummary(results, healthy, message, contracts.length > 0);

  for (const result of results) {
    console.log(
      `${result.name} (${result.id}): ${result.health}${result.paused ? " [PAUSED]" : ""}`,
    );
    if (result.output) {
      console.log(result.output);
    }
  }

  if (contracts.length === 0) {
    console.log("No mainnet deployments configured; reporting healthy.");
  }

  process.exit(0);
}

module.exports = {
  evaluateProbe,
  probeValue,
  probeAvailability,
  boolValue,
  inferCategory,
  parseLiveMainnet,
  resolveContracts,
  PROBES_BY_CATEGORY,
};