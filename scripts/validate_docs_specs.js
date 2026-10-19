const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertFile(relativePath) {
  assert(
    fs.existsSync(path.join(root, relativePath)),
    `Missing required file: ${relativePath}`,
  );
}

function assertContains(relativePath, snippets) {
  const content = read(relativePath);
  for (const snippet of snippets) {
    assert(
      content.includes(snippet),
      `${relativePath} must include ${JSON.stringify(snippet)}`,
    );
  }
}

function assertDocumentedLinks() {
  const index = read("DOCUMENTATION_INDEX.md");
  for (const target of [
    "docs/docs-specs-ci-coverage.md",
    "docs/troubleshooting-guide.md",
    "docs/api-reference-generation.md",
    "docs/github-action-support-matrix.md",
  ]) {
    assert(
      index.includes(target),
      `DOCUMENTATION_INDEX.md must link ${target}`,
    );
  }
}

function assertWorkflowCoverage() {
  const workflow = read(".github/workflows/ci.yml");
  assert(
    workflow.includes("Docs/specs integration coverage"),
    ".github/workflows/ci.yml must define docs/specs integration coverage",
  );
  assert(
    workflow.includes("npm run docs:specs:check"),
    ".github/workflows/ci.yml must run npm run docs:specs:check",
  );
}

function assertPackageScript() {
  const pkg = JSON.parse(read("package.json"));
  assert(
    pkg.scripts &&
      pkg.scripts["docs:specs:check"] === "node scripts/validate_docs_specs.js",
    "package.json must define scripts.docs:specs:check",
  );
}

function assertOwnerDocs() {
  assertContains("docs/docs-specs-ci-coverage.md", [
    "# Docs and Specs CI Coverage",
    "## Owner modules/files",
    "docs/soroban-deployment.md",
    "docs/ci-cd-setup.md",
    "docs/QUICK_REFERENCE.md",
    "docs/github-action-support-matrix.md",
    "scripts/action_inputs.py",
    "specs/sep41_token_total_supply.tla",
    "## Integration/e2e coverage contract",
    "npm run docs:specs:check",
    "## Stable output policy",
  ]);

  assertContains("docs/troubleshooting-guide.md", [
    "# Troubleshooting Guide",
    "## Module boundaries",
    "docs/soroban-deployment.md",
    "docs/ci-cd-setup.md",
    "docs/QUICK_REFERENCE.md",
    "## Contributor checklist",
  ]);

  assertContains("docs/api-reference-generation.md", [
    "# API Reference Generation",
    "## Canonical command",
    "cargo doc --workspace --no-deps",
    "make docs",
    "## Contribution notes",
    "## Output stability",
  ]);

  assertContains("docs/github-action-support-matrix.md", [
    "# GitHub Action Support Matrix",
    "## Supported runners",
    "## Compatibility matrix",
    "## Input validation and error messages",
    "Invalid Input",
    "Sanctifier action input error:",
    'python -m unittest discover -s tests/action -p "test_*.py"',
    "## Output stability",
  ]);
}

function assertActionInputValidation() {
  const action = read("action.yml");
  for (const input of [
    "path",
    "version",
    "min-severity",
    "format",
    "upload-sarif",
    "sarif-output",
    "debug",
  ]) {
    assert(
      action.includes(`${input}:`),
      `action.yml must define input ${input}`,
    );
  }

  const helper = read("scripts/action_inputs.py");
  for (const errorSnippet of [
    "format must be one of",
    "min-severity must be one of",
    "must not be empty",
    "must not contain control characters",
    "must not start with '-'",
    "must be relative to the checked-out repository",
    "must not contain '..' path traversal segments",
    "contains unsupported characters",
    "does not exist in the checked-out repository",
    "::error title=Invalid Input::Sanctifier action input error:",
  ]) {
    assert(
      helper.includes(errorSnippet),
      `scripts/action_inputs.py must keep validation error ${JSON.stringify(
        errorSnippet,
      )}`,
    );
  }

  const tests = read("tests/action/test_action_inputs.py");
  for (const testSnippet of [
    "test_rejects_unknown_format",
    "test_rejects_invalid_severity",
    "test_rejects_missing_scan_path",
    "test_rejects_path_traversal",
    "test_main_reports_invalid_input_as_github_error",
  ]) {
    assert(
      tests.includes(testSnippet),
      `tests/action/test_action_inputs.py must cover ${testSnippet}`,
    );
  }
}

function assertSpecCoverage() {
  const spec = read("specs/sep41_token_total_supply.tla");
  for (const symbol of [
    "SupplyInvariant",
    "TypeOK",
    "Spec ==",
    "TransferAtoB",
    "MintToA",
  ]) {
    assert(spec.includes(symbol), `SEP-41 spec must keep ${symbol} coverage`);
  }
}

for (const file of [
  "docs/docs-specs-ci-coverage.md",
  "docs/troubleshooting-guide.md",
  "docs/api-reference-generation.md",
  "docs/soroban-deployment.md",
  "docs/ci-cd-setup.md",
  "docs/QUICK_REFERENCE.md",
  "docs/github-action-support-matrix.md",
  "specs/sep41_token_total_supply.tla",
  "action.yml",
  "scripts/action_inputs.py",
  "tests/action/test_action_inputs.py",
]) {
  assertFile(file);
}

assertPackageScript();
assertDocumentedLinks();
assertWorkflowCoverage();
assertOwnerDocs();
assertActionInputValidation();
assertSpecCoverage();

console.log("docs/specs integration coverage passed");
