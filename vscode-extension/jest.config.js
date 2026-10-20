'use strict';

/**
 * Jest configuration for the Sanctifier VS Code extension.
 *
 * Test organization:
 * - Unit tests live in src/test/*.test.ts alongside their fixtures (src/test/fixtures/)
 * - The testMatch pattern picks up all *.test.ts files under src/
 * - Tests use both inline test data (wrap() helper) and external fixtures for complex cases
 *
 * Fixture-based tests (src/test/analyzer.test.ts) are preferred for:
 * - Complex multi-line contracts
 * - Realistic error scenarios
 * - Regression test cases
 *
 * Inline tests are acceptable for:
 * - Simple edge cases
 * - Single-assertion unit tests
 * - Quick-fix helper functions
 */

/** @type {import('jest').Config} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/src/**/*.test.ts'],
  moduleNameMapper: {
    '^vscode$': '<rootDir>/src/__mocks__/vscode.ts',
  },
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: {
          module: 'commonjs',
          target: 'ES2022',
          lib: ['ES2022'],
          strict: true,
          esModuleInterop: true,
          skipLibCheck: true,
        },
      },
    ],
  },
};
