import "server-only";

import { readdir, readFile, stat } from "fs/promises";
import path from "path";
import {
  countPatterns,
  deriveTags,
  riskFromCounts,
  totalFindings,
  type ContractInfo,
  type PatternCounts,
} from "./contracts-model";

const CONTRACT_NAME_RE = /^[A-Za-z0-9_-]+$/;

/** The workspace `contracts/` directory, resolved relative to the frontend package. */
export function contractsRoot(): string {
  return path.resolve(process.cwd(), "..", "contracts");
}

async function exists(p: string): Promise<boolean> {
  try {
    await stat(p);
    return true;
  } catch {
    return false;
  }
}

async function collectRustFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) return collectRustFiles(full);
      return entry.name.endsWith(".rs") ? [full] : [];
    }),
  );
  return files.flat();
}

function readCargoDescription(cargoToml: string): string | null {
  const match = cargoToml.match(/^description\s*=\s*"([^"]+)"/m);
  return match ? match[1] : null;
}

function readReadmeSummary(readme: string): string | null {
  const line = readme
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l && !l.startsWith("#") && !l.startsWith("[") && !l.startsWith("!") && !l.startsWith("|"));
  return line ? line.slice(0, 200) : null;
}

async function readOptional(p: string): Promise<string | null> {
  try {
    return await readFile(p, "utf-8");
  } catch {
    return null;
  }
}

async function loadContract(root: string, name: string): Promise<ContractInfo | null> {
  const dir = path.join(root, name);
  const srcDir = path.join(dir, "src");
  if (!(await exists(path.join(dir, "Cargo.toml"))) || !(await exists(srcDir))) return null;

  const [cargo, readme, rustFiles] = await Promise.all([
    readOptional(path.join(dir, "Cargo.toml")),
    readOptional(path.join(dir, "README.md")),
    collectRustFiles(srcDir),
  ]);

  const counts: PatternCounts = { panics: 0, unsafeBlocks: 0, rawInvokes: 0 };
  let linesOfCode = 0;
  for (const file of rustFiles) {
    const source = await readFile(file, "utf-8");
    linesOfCode += source.split("\n").filter((l) => l.trim().length > 0).length;
    const c = countPatterns(source);
    counts.panics += c.panics;
    counts.unsafeBlocks += c.unsafeBlocks;
    counts.rawInvokes += c.rawInvokes;
  }

  const description =
    (cargo && readCargoDescription(cargo)) ||
    (readme && readReadmeSummary(readme)) ||
    `Soroban contract in contracts/${name}`;

  const extras: string[] = [];
  if (await exists(path.join(dir, "fuzz"))) extras.push("fuzzed");
  if (await exists(path.join(dir, "tests"))) extras.push("tested");

  return {
    name,
    path: `contracts/${name}`,
    description,
    tags: deriveTags(name, description, extras),
    risk: riskFromCounts(counts),
    findingCount: totalFindings(counts),
    linesOfCode,
  };
}

/** List every contract crate under `contracts/`. Returns `[]` if the directory is unavailable. */
export async function loadContracts(): Promise<ContractInfo[]> {
  const root = contractsRoot();
  let names: string[];
  try {
    const entries = await readdir(root, { withFileTypes: true });
    names = entries.filter((e) => e.isDirectory() && CONTRACT_NAME_RE.test(e.name)).map((e) => e.name);
  } catch {
    return [];
  }

  const loaded = await Promise.all(names.map((n) => loadContract(root, n)));
  return loaded.filter((c): c is ContractInfo => c !== null).sort((a, b) => a.name.localeCompare(b.name));
}

export async function loadContractByName(name: string): Promise<ContractInfo | null> {
  if (!CONTRACT_NAME_RE.test(name)) return null;
  return loadContract(contractsRoot(), name);
}

/** Return the main source file (`src/lib.rs`) for a known contract, or null. */
export async function loadContractSource(name: string): Promise<string | null> {
  if (!CONTRACT_NAME_RE.test(name)) return null;
  return readOptional(path.join(contractsRoot(), name, "src", "lib.rs"));
}
