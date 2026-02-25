# ADR 003: Focus on Static Analysis Over Runtime Analysis

## Status

Accepted

## Context

When building a security analysis tool for smart contracts, there are generally two approaches:
1. **Static Analysis:** Examining the source code or compiled binary (WASM) without executing it. Techniques include pattern matching, control flow analysis, and data flow analysis.
2. **Runtime (Dynamic) Analysis:** Executing the code (often in a sandboxed or instrumented environment) with various inputs (e.g., fuzzing, symbolic execution) to observe behavior and identify vulnerabilities during runtime.

We need to decide the primary focus of Sanctifier to scope the project effectively and deliver value quickly to the Soroban developer community.

## Decision

Sanctifier will focus primarily on **Static Analysis** of compiled WebAssembly (WASM) binaries and (optionally) Rust source code before deployment. 

While dynamic analysis tools (like fuzzers) are valuable, they require entirely different infrastructure, complex execution environments, and potentially longer execution times. Sanctifier will aim to be a fast, deterministic tool that fits easily into CI/CD pipelines and local development workflows.

## Consequences

**Positive:**
* **Speed:** Static analysis is generally much faster than dynamic analysis, providing rapid feedback to developers.
* **Coverage:** Static analysis can theoretically examine all execution paths, whereas dynamic analysis is limited by the test cases or fuzzing inputs provided.
* **Integration:** Easier to integrate into standard CI/CD pipelines as a linter or pre-deployment check.
* **Simpler Infrastructure:** No need to build or maintain a complex soroban-environment execution harness within Sanctifier itself.

**Negative:**
* **False Positives:** Static analysis tools often struggle with complex runtime state and can report false positives that a dynamic execution would prove impossible.
* **False Negatives:** Complex vulnerabilities that rely on specific timing, state changes, or deep cryptographic manipulations might be missed without actual execution.
* **Limited Scope:** We will not be building a fuzzer or symbolic execution engine as part of the core Sanctifier project in the near term.
