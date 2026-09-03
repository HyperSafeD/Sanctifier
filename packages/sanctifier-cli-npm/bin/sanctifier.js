#!/usr/bin/env node
const { spawnSync } = require("child_process");
const { existsSync, mkdirSync, chmodSync, writeFileSync, unlinkSync } = require("fs");
const path = require("path");
const os = require("os");

const BINARY_NAME = "sanctifier";
const REPO = "HyperSafeD/Sanctifier";
const CACHE_DIR = path.join(os.homedir(), ".sanctifier", "bin");
const GITHUB_API = "https://api.github.com";

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "linux" && arch === "x64") return "linux-amd64-musl";
  if (platform === "darwin" && arch === "x64") return "macos-amd64";
  if (platform === "darwin" && arch === "arm64") return "macos-arm64";
  if (platform === "win32" && arch === "x64") return "windows-amd64";

  throw new Error(
    `Unsupported platform: ${platform}/${arch}. Sanctifier currently supports macOS (x64/arm64) and Linux (x64).`
  );
}

function getBinaryName(platform) {
  if (platform.startsWith("windows")) return `${BINARY_NAME}-${platform}.exe`;
  return `${BINARY_NAME}-${platform}`;
}

async function fetchLatestVersion() {
  const url = `${GITHUB_API}/repos/${REPO}/releases/latest`;
  try {
    const res = await fetch(url, {
      headers: { "User-Agent": "@hypersafed/sanctifier-cli" },
    });
    if (!res.ok) throw new Error(`GitHub API returned ${res.status}`);
    const data = await res.json();
    return data.tag_name.replace(/^v/, "");
  } catch (err) {
    const result = spawnSync("curl", ["-fsSL", url], {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    if (result.status !== 0)
      throw new Error(
        `Failed to fetch latest version. ${result.stderr || err.message}`
      );
    const data = JSON.parse(result.stdout);
    return data.tag_name.replace(/^v/, "");
  }
}

async function downloadBinary(version, platform, dest) {
  const binaryName = getBinaryName(platform);
  const downloadUrl = `https://github.com/${REPO}/releases/download/v${version}/${binaryName}`;

  const res = await fetch(downloadUrl, {
    headers: { "User-Agent": "@hypersafed/sanctifier-cli" },
    redirect: "follow",
  });

  if (!res.ok) {
    throw new Error(
      `Failed to download binary: HTTP ${res.status} from ${downloadUrl}`
    );
  }

  if (!existsSync(path.dirname(dest))) {
    mkdirSync(path.dirname(dest), { recursive: true });
  }

  const buffer = Buffer.from(await res.arrayBuffer());
  writeFileSync(dest, buffer);
  chmodSync(dest, 0o755);
}

async function ensureBinary() {
  const platform = getPlatform();
  const binaryName =
    os.platform() === "win32"
      ? `${BINARY_NAME}.exe`
      : BINARY_NAME;
  const cachedBinary = path.join(CACHE_DIR, binaryName);

  if (existsSync(cachedBinary)) return cachedBinary;

  const version = await fetchLatestVersion();
  await downloadBinary(version, platform, cachedBinary);
  return cachedBinary;
}

async function main() {
  let binaryPath;
  try {
    binaryPath = await ensureBinary();
  } catch (err) {
    console.error(
      `sanctifier: failed to download binary. ${err.message}`
    );
    console.error("Install via cargo: cargo install sanctifier-cli");
    console.error("Install via brew:  brew install HyperSafeD/sanctifier/sanctifier");
    process.exit(1);
  }

  const args = process.argv.slice(2);
  const result = spawnSync(binaryPath, args, { stdio: "inherit" });

  if (result.error) {
    if (result.error.code === "ENOENT") {
      console.error(
        `sanctifier: binary not found at ${binaryPath}. Cache may be corrupt.`
      );
      try {
        unlinkSync(binaryPath);
      } catch (_) {}
      process.exit(1);
    }
    console.error(`sanctifier: failed to run binary. ${result.error.message}`);
    process.exit(1);
  }

  process.exit(result.status || 0);
}

main().catch((err) => {
  console.error(`sanctifier: unexpected error. ${err.message}`);
  process.exit(1);
});
