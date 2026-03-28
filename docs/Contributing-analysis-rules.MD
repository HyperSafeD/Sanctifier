# Contributing Static Analysis Rules & Formal Verification Templates

> A guide for researchers and developers who want to extend the project's analysis capabilities.

---

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Contributing a Static Analysis Rule](#contributing-a-static-analysis-rule)
4. [Contributing a Formal Verification Template](#contributing-a-formal-verification-template)
5. [Testing & Validation](#testing--validation)
6. [Contribution Workflow](#contribution-workflow)
7. [Review Checklist](#review-checklist)
8. [Examples](#examples)
9. [Getting Help](#getting-help)

---

## Overview

This guide explains how researchers and developers can contribute:

- **Static analysis rules** — patterns or checks that detect issues in source code without executing it.
- **Formal verification templates** — reusable specifications used to mathematically prove or disprove properties of a program.

Contributions of both types follow the same general workflow: write, test, document, and submit a pull request.

---

## Project Structure

```
project-root/
├── rules/
│   ├── security/          # Security-focused static analysis rules
│   ├── quality/           # Code quality rules
│   └── custom/            # Community-contributed rules
├── verification/
│   ├── templates/         # Formal verification templates
│   └── specs/             # Standalone specification files
├── tests/
│   ├── rules/             # Unit tests for each rule
│   └── verification/      # Tests for verification templates
├── docs/
│   └── contributing/      # Contributor documentation (you are here)
└── CONTRIBUTING.md        # Top-level contribution guidelines
```

> **Note:** If your project uses a different layout, adjust the paths above to match. The principles remain the same.

---

## Contributing a Static Analysis Rule

### What is a Static Analysis Rule?

A rule describes a pattern to detect in source code — for example, use of a deprecated function, a potential null dereference, or an insecure API call.

### Step 1 — Choose the Right Category

| Category    | Use for...                                              |
|-------------|----------------------------------------------------------|
| `security/` | Vulnerabilities, insecure patterns, input validation     |
| `quality/`  | Code smells, complexity issues, style violations         |
| `custom/`   | Experimental or domain-specific rules                    |

### Step 2 — Define the Rule

Create a new file in the appropriate directory. Most projects define rules in one of these formats:

**YAML-based rule (common in tools like Semgrep):**

```yaml
# rules/security/avoid-hardcoded-secrets.yaml
rules:
  - id: avoid-hardcoded-secrets
    patterns:
      - pattern: |
          $VAR = "..."
    message: >
      Possible hardcoded secret detected in '$VAR'.
      Consider using environment variables or a secrets manager.
    languages: [python, javascript, go]
    severity: ERROR
    metadata:
      category: security
      confidence: MEDIUM
      references:
        - https://owasp.org/www-community/vulnerabilities/Use_of_hard-coded_credentials
```

**JSON-based rule:**

```json
{
  "id": "no-eval",
  "description": "Avoid use of eval() as it can execute arbitrary code.",
  "pattern": "eval(*)",
  "severity": "high",
  "language": "javascript",
  "fix": "Replace eval() with safer alternatives like JSON.parse() or Function constructors."
}
```

### Step 3 — Required Fields

Every rule **must** include:

| Field         | Description                                              |
|---------------|----------------------------------------------------------|
| `id`          | Unique, kebab-case identifier (e.g., `no-sql-injection`) |
| `description` | Clear, one-sentence summary of what the rule detects     |
| `severity`    | One of: `ERROR`, `WARNING`, `INFO`                       |
| `language`    | Target language(s)                                       |
| `message`     | Human-readable message shown to the developer            |

**Optional but recommended:**

| Field        | Description                                      |
|--------------|--------------------------------------------------|
| `fix`        | Suggested remediation                            |
| `references` | Links to relevant standards or documentation     |
| `metadata`   | Tags, confidence level, CWE/CVE identifiers      |

---

## Contributing a Formal Verification Template

### What is a Formal Verification Template?

A template encodes a program property — such as memory safety, absence of race conditions, or correct API usage — in a formal specification language (e.g., TLA+, Coq, Dafny, ACSL). Templates are reusable across similar codebases.

### Step 1 — Choose a Specification Language

Use the language that best fits the property you want to verify:

| Language | Best for...                              |
|----------|------------------------------------------|
| TLA+     | Concurrent systems, distributed protocols |
| Dafny    | Imperative code with pre/postconditions  |
| Coq      | Mathematical proofs, type-theoretic properties |
| ACSL     | C code formal contracts                  |
| Why3     | Deductive verification across languages  |

### Step 2 — Create the Template File

Place your template in `verification/templates/` with a descriptive filename.

**Example — Dafny template for a bounded buffer:**

```dafny
// verification/templates/bounded-buffer.dfy
// Template: Bounded Buffer Safety
// Verifies that buffer reads/writes never exceed capacity.

class BoundedBuffer<T> {
  var contents: array<T>
  var size: int
  ghost var capacity: int

  predicate Valid()
    reads this
  {
    0 <= size <= capacity &&
    contents.Length == capacity
  }

  method Push(item: T)
    requires Valid()
    requires size < capacity
    modifies this
    ensures Valid()
    ensures size == old(size) + 1
  {
    contents[size] := item;
    size := size + 1;
  }

  method Pop() returns (item: T)
    requires Valid()
    requires size > 0
    modifies this
    ensures Valid()
    ensures size == old(size) - 1
  {
    item := contents[size - 1];
    size := size - 1;
  }
}
```

### Step 3 — Required Template Metadata

Add a header comment block to every template:

```
// Template ID:   bounded-buffer
// Description:   Verifies read/write safety for a fixed-capacity buffer
// Language:      Dafny
// Author:        Your Name <email@example.com>
// Date:          YYYY-MM-DD
// Verified with: Dafny v4.x
// Properties:    Memory safety, bounds checking
// References:    <optional links>
```

---

## Testing & Validation

Before submitting, all contributions must pass tests.

### Testing Static Analysis Rules

1. **Create test fixtures** in `tests/rules/<rule-id>/`:
   - `should-match/` — code that **should** trigger the rule
   - `should-not-match/` — code that **should not** trigger the rule

```
tests/rules/avoid-hardcoded-secrets/
├── should-match/
│   ├── example1.py        # password = "hunter2"
│   └── example2.js        // const apiKey = "sk-1234abcd"
└── should-not-match/
    ├── example1.py        # password = os.getenv("PASSWORD")
    └── example2.js        // const apiKey = process.env.API_KEY
```

2. **Run the test suite:**

```bash
# Run tests for a specific rule
make test-rule RULE=avoid-hardcoded-secrets

# Run all rule tests
make test-rules
```

3. **Check for false positives and false negatives** — a good rule has:
   - High **precision** (few false positives)
   - High **recall** (few missed real issues)

### Testing Formal Verification Templates

1. **Verify the template compiles and proves cleanly:**

```bash
# Dafny example
dafny verify verification/templates/bounded-buffer.dfy

# TLA+ example
tlc verification/templates/MyProtocol.tla
```

2. **Add a negative test** — a version of the code that deliberately violates the property, to confirm verification catches it.

3. **Document any assumptions** the template makes (e.g., single-threaded execution, finite inputs).

---

## Contribution Workflow

```
1. Fork the repository
        ↓
2. Create a feature branch
   git checkout -b rule/avoid-hardcoded-secrets
        ↓
3. Write your rule or template
        ↓
4. Add tests (should-match + should-not-match)
        ↓
5. Run the full test suite locally
   make test
        ↓
6. Commit with a descriptive message
   git commit -m "feat(rules): add avoid-hardcoded-secrets rule"
        ↓
7. Open a Pull Request against main
        ↓
8. Respond to reviewer feedback
        ↓
9. Merge 🎉
```

### Branch Naming Convention

| Type                       | Branch name pattern                        |
|----------------------------|--------------------------------------------|
| New static analysis rule   | `rule/<rule-id>`                           |
| New verification template  | `verify/<template-name>`                   |
| Fix to an existing rule    | `fix/rule-<rule-id>`                       |
| Documentation update       | `docs/<topic>`                             |

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(rules): add null-pointer-dereference detection rule
fix(verify): correct precondition in bounded-buffer template
docs: update contribution guide with ACSL examples
```

---

## Review Checklist

Use this checklist before submitting your PR. Reviewers will also use it.

### For Static Analysis Rules

- [ ] Rule has a unique, descriptive `id`
- [ ] `severity` is appropriate and justified
- [ ] `message` is clear and actionable for developers
- [ ] At least 2 `should-match` test cases provided
- [ ] At least 2 `should-not-match` test cases provided
- [ ] No obvious false positives on common code patterns
- [ ] References to standards or CVEs included where relevant
- [ ] Rule documented in `docs/rules/<rule-id>.md` (if required by project)

### For Formal Verification Templates

- [ ] Template compiles and verifies without errors
- [ ] Header metadata block is complete
- [ ] Properties being verified are clearly stated
- [ ] A negative test (intentional violation) is included
- [ ] Assumptions and limitations are documented
- [ ] Template is general enough to be reused across codebases

---

## Examples

### Example 1: Simple Static Analysis Rule (Python — Use of `assert` in Production)

**Rule file:** `rules/quality/no-production-assert.yaml`

```yaml
rules:
  - id: no-production-assert
    pattern: assert $CONDITION
    message: >
      Avoid using 'assert' in production code. Assertions can be
      disabled with the -O flag, making this check ineffective.
      Use explicit if/raise patterns instead.
    languages: [python]
    severity: WARNING
    metadata:
      category: quality
      fix: "Replace `assert x` with `if not x: raise ValueError(...)`"
```

**Test — should match:** `assert user is not None`

**Test — should not match:** `if user is None: raise ValueError("User required")`

---

### Example 2: Formal Verification Template (TLA+ — Mutual Exclusion)

```tla
---- MODULE MutualExclusion ----
EXTENDS Naturals, TLC

CONSTANT N  \* Number of processes

ASSUME N > 1

Processes == 1..N

VARIABLES state

TypeOK == state \in [Processes -> {"idle", "waiting", "critical"}]

Init == state = [p \in Processes |-> "idle"]

\* At most one process in critical section at any time
MutualExclusion == \A p, q \in Processes :
  (p # q) => ~(state[p] = "critical" /\ state[q] = "critical")

\* ... (transitions defined here)

====
```

---

## Getting Help

If you have questions about contributing:

- **Open a Discussion** on GitHub for design questions before writing code.
- **Check existing rules/templates** for style and structure guidance.
- **Tag your issue** with `static-analysis` or `formal-verification` for faster routing.
- **Reach out to maintainers** listed in `CODEOWNERS` or `MAINTAINERS.md`.

We welcome contributions from researchers at all levels. If you're unsure whether your rule idea is a good fit, open an issue first — maintainers are happy to give early feedback.

---

*Last updated: March 2026 | License: see project root LICENSE file*