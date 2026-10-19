const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const workspace = process.env.GITHUB_WORKSPACE || process.cwd();
const liveTestnetPath = path.join(workspace, "LIVE_TESTNET.md");
const monitorDir = path.join(workspace, "monitor");

function parseContracts(markdown) {
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
      };
    })
    .filter(Boolean);
}

function runHealthCheck(contract) {
  const startedAt = new Date().toISOString();
  // Use `stellar contract fetch` to verify the contract is deployed and
  // reachable.  Invoking a specific entry-point (e.g. `health_check`) fails
  // for contracts that don't export that function, which was the root cause
  // of the recurring #909 issue reports for the Vulnerable Contract demo.
  const result = spawnSync(
    "stellar",
    [
      "contract",
      "fetch",
      "--id",
      contract.id,
      "--network",
      "testnet",
    ],
    { encoding: "utf8", timeout: 30_000 },
  );

  const stdout = result.stdout.trim();
  const stderr = result.stderr.trim();
  const output = [stdout, stderr].filter(Boolean).join("\n");

  return {
    ...contract,
    checkedAt: startedAt,
    healthy: result.status === 0,
    exitCode: result.status,
    output,
  };
}

function writeGitHubOutput(results) {
  const githubOutput = process.env.GITHUB_OUTPUT;

  if (!githubOutput) {
    return;
  }

  const failures = results.filter((result) => !result.healthy);
  fs.appendFileSync(githubOutput, `healthy=${failures.length === 0}\n`);
  fs.appendFileSync(githubOutput, `failures=${failures.length}\n`);
  fs.appendFileSync(githubOutput, `total=${results.length}\n`);
}

function writeSummary(results, badge) {
  const githubStepSummary = process.env.GITHUB_STEP_SUMMARY;

  if (!githubStepSummary) {
    return;
  }

  const lines = [
    "## Testnet health monitor",
    "",
    `Badge: ${badge.message}`,
    "",
    "| Contract | Address | Result |",
    "|---|---|---|",
    ...results.map((result) => {
      const status = result.healthy ? "healthy" : "failed";
      return `| ${result.name} | \`${result.id}\` | ${status} |`;
    }),
    "",
  ];

  fs.appendFileSync(githubStepSummary, lines.join("\n"));
}

const markdown = fs.readFileSync(liveTestnetPath, "utf8");
const contracts = parseContracts(markdown);

if (contracts.length === 0) {
  throw new Error("No contract IDs were found in LIVE_TESTNET.md");
}

fs.mkdirSync(monitorDir, { recursive: true });

const results = contracts.map(runHealthCheck);
const failures = results.filter((result) => !result.healthy);
const badge = {
  schemaVersion: 1,
  label: "testnet monitor",
  message:
    failures.length === 0
      ? `${results.length}/${results.length} healthy`
      : `${results.length - failures.length}/${results.length} healthy`,
  color: failures.length === 0 ? "brightgreen" : "red",
};

fs.writeFileSync(
  path.join(monitorDir, "results.json"),
  JSON.stringify(
    { checkedAt: new Date().toISOString(), contracts: results },
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

writeGitHubOutput(results);
writeSummary(results, badge);

for (const result of results) {
  const status = result.healthy ? "healthy" : "failed";
  console.log(`${result.name} (${result.id}): ${status}`);
  if (!result.healthy && result.output) {
    console.log(result.output);
  }
}
