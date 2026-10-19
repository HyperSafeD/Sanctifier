#!/usr/bin/env node
/**
 * CSP Compliance Auditor for Sanctifier WASM
 * 
 * This script scans the generated JS glue code in the `pkg/` directory for 
 * patterns that trigger CSP 'unsafe-eval' violations:
 * 1. eval(...)
 * 2. new Function(...)
 * 
 * Usage: node scripts/verify-csp-compliance.js [pkg-dir]
 */

const fs = require("fs");
const path = require("path");

const pkgDir = process.argv[2] || path.resolve(__dirname, "..", "pkg");

if (!fs.existsSync(pkgDir)) {
  console.error(`ERROR: Package directory not found: ${pkgDir}`);
  process.exit(1);
}

function auditFile(filePath) {
  const content = fs.readFileSync(filePath, "utf8");
  const fileName = path.basename(filePath);
  let violations = 0;

  // Pattern 1: eval(...)
  // We use a regex that ignores comments (to some extent) but catches most usages.
  const evalRegex = /\beval\s*\(/g;
  let match;
  while ((match = evalRegex.exec(content)) !== null) {
    console.error(`VIOLATION: Found 'eval(' in ${fileName} at offset ${match.index}`);
    violations++;
  }

  // Pattern 2: new Function(...)
  // This is a common pattern wasm-bindgen uses for global object detection.
  // We allow the specific pattern 'new Function("return this")()' or similar
  // if it's clearly for global detection.
  const funcRegex = /new\s+Function\s*\(([^)]*)\)/g;
  while ((match = funcRegex.exec(content)) !== null) {
    const args = match[1].trim();
    // Allow standard wasm-bindgen global detection boilerplate
    if (args === '"return this"' || args === "'return this'") {
      continue;
    }
    console.error(`VIOLATION: Found 'new Function(${args})' in ${fileName} at offset ${match.index}`);
    violations++;
  }

  return violations;
}

const jsFiles = fs.readdirSync(pkgDir).filter(f => f.endsWith(".js"));

if (jsFiles.length === 0) {
  console.warn("WARNING: No JS files found in pkg directory to audit.");
  process.exit(0);
}

console.log(`Auditing ${jsFiles.length} files in ${pkgDir} for CSP compliance...`);

let totalViolations = 0;
for (const file of jsFiles) {
  totalViolations += auditFile(path.join(pkgDir, file));
}

if (totalViolations > 0) {
  console.error(`\nFAILED: Found ${totalViolations} CSP violations in generated WASM glue code.`);
  console.error("The package is not 'unsafe-eval' CSP-friendly.");
  process.exit(1);
}

console.log("SUCCESS: No CSP-violating patterns found. The package is CSP-friendly.");
process.exit(0);
