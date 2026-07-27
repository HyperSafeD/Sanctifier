# E2E Test Fixtures

This directory contains test fixtures for end-to-end regression testing.

## Required Fixtures

### test-contract.wasm
A simple Soroban contract with known security characteristics for testing the full scan-to-deploy flow.

**To generate:**
```bash
cd contracts/test-samples/reentrancy-example
soroban contract build
cp target/wasm32-unknown-unknown/release/*.wasm ../../e2e/fixtures/test-contract.wasm
```

**Alternatively**, use any sample contract:
```bash
soroban contract init test-contract
cd test-contract
soroban contract build
cp target/wasm32-unknown-unknown/release/*.wasm ../e2e/fixtures/test-contract.wasm
```

### expected-sarif.json
Expected SARIF output format for validation in E2E tests.

Example structure:
```json
{
  "version": "2.1.0",
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "runs": [{
    "tool": {
      "driver": {
        "name": "Sanctifier",
        "version": "1.0.0"
      }
    },
    "results": []
  }]
}
```

## Usage in Tests

E2E tests reference these fixtures via relative paths:
```typescript
const testContractPath = path.join(__dirname, '../fixtures/test-contract.wasm');
```

## Maintenance

- Update fixtures when contract ABI changes
- Regenerate expected outputs when scanner rules change
- Keep fixtures minimal to ensure fast test execution
