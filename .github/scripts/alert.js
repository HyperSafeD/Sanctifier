#!/usr/bin/env node
/**
 * alert.js — shared webhook alerting utility for GitHub Actions.
 *
 * Sends an incident/health notification to one or more HTTPS webhook
 * endpoints (Slack, Discord or any generic incoming-webhook target) from a
 * scheduled monitor or operational workflow.  Companion to the `webhook`
 * module in `tooling/sanctifier-cli`; payloads and the HMAC signing scheme
 * (header `X-Sanctifier-Signature-256`, value `sha256=<hex>`) match that
 * implementation so a single receiver can authenticate both channels.
 *
 * Usage:
 *   node .github/scripts/alert.js \
 *     --event      <event-type>          # e.g. health.failed / deploy.completed
 *     --title      <notification title>
 *     --message    <notification body>
 *     --severity   <info|warning|high|critical>   (default: info)
 *     [--field KEY=VALUE ...]            # extra structured fields
 *     [--run-url <url>]                  # link to the workflow run
 *     [--dry-run]                        # print payloads without sending
 *
 * Configuration (environment):
 *   ALERT_WEBHOOK_URLS       Space/comma/newline-separated HTTPS webhook URLs.
 *   ALERT_WEBHOOK_SECRET     Optional HMAC-SHA256 secret; when set every
 *                            request carries an `X-Sanctifier-Signature-256`
 *                            header so receivers can verify authenticity.
 *   ALERT_WEBHOOK_FAIL_FAST  Set to "true" to exit non-zero if delivery to
 *                            every URL fails after retries (default:
 *                            best-effort — monitoring must never fail because
 *                            a notification endpoint is down).
 *
 * Threat model (mirrors tooling/sanctifier-cli/src/commands/webhook.rs):
 *   T1 spoofed payloads       -> HMAC-SHA256 signature header
 *   T2 transient failures     -> exponential backoff, 3 attempts, cap 30s
 *   T3 slow endpoints         -> 10s per-request timeout
 *   T4 plaintext transmission -> HTTPS-only URLs enforced below
 *   T5 secret leakage         -> the secret is never logged, only its digest
 */

const crypto = require("node:crypto");
const fs = require("node:fs");

const DEFAULT_MAX_ATTEMPTS = 3;
const REQUEST_TIMEOUT_MS = 10_000;

function parseArgs(argv) {
  const args = { fields: [] };
  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i];
    const next = () => argv[++i];
    switch (token) {
      case "--event":
        args.event = next();
        break;
      case "--title":
        args.title = next();
        break;
      case "--message":
        args.message = next();
        break;
      case "--severity":
        args.severity = next();
        break;
      case "--field": {
        const field = next();
        const idx = field.indexOf("=");
        if (idx < 0) {
          throw new Error(`--field expects KEY=VALUE, got: ${field}`);
        }
        args.fields.push([field.slice(0, idx), field.slice(idx + 1)]);
        break;
      }
      case "--run-url":
        args.runUrl = next();
        break;
      case "--dry-run":
        args.dryRun = true;
        break;
      default:
        throw new Error(`unknown argument: ${token}`);
    }
  }

  if (!args.title) {
    throw new Error("Missing required --title argument");
  }
  return args;
}

function webhookUrls() {
  return (process.env.ALERT_WEBHOOK_URLS || "")
    .split(/[\s,]+/)
    .map((url) => url.trim())
    .filter(Boolean);
}

function validateUrl(url) {
  if (!url.startsWith("https://")) {
    throw new Error(
      `webhook URL '${url}' must use HTTPS to prevent plaintext secret transmission`,
    );
  }
}

function classifyProvider(url) {
  if (
    url.includes("discord.com/api/webhooks") ||
    url.includes("discordapp.com/api/webhooks")
  ) {
    return "discord";
  }
  if (url.includes("hooks.slack.com")) {
    return "slack";
  }
  return "generic";
}

function severityColor(severity) {
  switch (severity) {
    case "critical":
      return "#d92d20";
    case "high":
      return "#f79009";
    case "warning":
      return "#fde047";
    default:
      return "#17b26a";
  }
}

function buildPayload(url, args) {
  const provider = classifyProvider(url);
  const fields = [
    { title: "Event", value: args.event || "notification", short: true },
    ...args.fields.map(([key, value]) => ({ title: key, value, short: true })),
    { title: "Timestamp", value: new Date().toISOString(), short: true },
  ];
  if (args.runUrl) {
    fields.push({ title: "Workflow run", value: args.runUrl, short: true });
  }

  if (provider === "discord") {
    const content = [
      `**${args.title}**`,
      args.message || "",
      args.runUrl ? `<${args.runUrl}>` : "",
    ]
      .filter(Boolean)
      .join("\n");
    return { content };
  }

  if (provider === "slack") {
    return {
      text: args.title,
      attachments: [
        {
          color: severityColor(args.severity || "info"),
          fields,
        },
      ],
    };
  }

  return {
    title: args.title,
    message: args.message || "",
    severity: args.severity || "info",
    event: args.event || "notification",
    timestamp_unix: String(Math.floor(Date.now() / 1000)),
    run_url: args.runUrl || null,
    fields: Object.fromEntries(args.fields),
  };
}

function hmacSignature(secret, body) {
  return `sha256=${crypto.createHmac("sha256", secret).update(body).digest("hex")}`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function deliver(url, body, secret) {
  const payload = Buffer.from(JSON.stringify(body), "utf8");
  const headers = { "Content-Type": "application/json" };
  if (secret) {
    headers["X-Sanctifier-Signature-256"] = hmacSignature(secret, payload);
  }

  console.log(`Notification: POST ${url}`);
  let lastError = null;
  for (let attempt = 1; attempt <= DEFAULT_MAX_ATTEMPTS; attempt += 1) {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
      const response = await fetch(url, {
        method: "POST",
        headers,
        body: payload,
        signal: controller.signal,
      });
      clearTimeout(timer);

      if (response.ok) {
        if (attempt > 1) {
          console.log(`Delivered ${url} after retry (attempt ${attempt}).`);
        }
        return;
      }
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error.name === "AbortError" ? "timeout (10s)" : error.message;
    }

    console.warn(
      `Webhook delivery failed for ${url} (${lastError}), attempt ${attempt}/${DEFAULT_MAX_ATTEMPTS}.`,
    );
    if (attempt < DEFAULT_MAX_ATTEMPTS) {
      await sleep(Math.min(1 << (attempt - 1), 30) * 1000);
    }
  }
  throw new Error(`${url}: ${lastError}`);
}

function writeSummary(args) {
  const githubStepSummary = process.env.GITHUB_STEP_SUMMARY;
  if (!githubStepSummary) {
    return;
  }
  const lines = [
    "## Alert notification",
    "",
    `**${args.title}**`,
    args.message || "",
    "",
    `- Severity: ${args.severity || "info"}`,
    `- Delivered to ${webhookUrls().length} webhook(s)`,
    "",
  ];
  fs.appendFileSync(githubStepSummary, lines.join("\n"));
}

async function main() {
  const args = parseArgs(process.argv);

  const urls = webhookUrls();
  if (urls.length === 0) {
    console.log("No ALERT_WEBHOOK_URLS configured — skipping webhook notification.");
    return;
  }

  for (const url of urls) {
    validateUrl(url);
  }

  const secret = process.env.ALERT_WEBHOOK_SECRET || null;
  const failures = [];

  for (const url of urls) {
    const body = buildPayload(url, args);
    if (args.dryRun) {
      const headers = { "Content-Type": "application/json" };
      if (secret) {
        headers["X-Sanctifier-Signature-256"] = hmacSignature(
          secret,
          Buffer.from(JSON.stringify(body), "utf8"),
        );
      }
      console.log(`[dry-run] ${url}`);
      console.log(`[dry-run] headers ${JSON.stringify(headers)}`);
      console.log(`[dry-run] body ${JSON.stringify(body, null, 2)}`);
      continue;
    }
    try {
      await deliver(url, body, secret);
      console.log(`Delivered alert to ${url}.`);
    } catch (error) {
      failures.push(error.message);
      console.error(`Alert delivery failed: ${error.message}`);
    }
  }

  writeSummary(args);

  if (failures.length === urls.length && process.env.ALERT_WEBHOOK_FAIL_FAST === "true") {
    throw new Error("All webhook deliveries failed after retries.");
  }
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}

module.exports = { buildPayload, hmacSignature, classifyProvider, validateUrl };