import assert from 'node:assert/strict';
import test from 'node:test';
import {
  explainFinding,
  formatFindingExplanation,
  normalizeFindingCode,
} from '../src/findings.js';
import { formatLatestFindings } from '../src/latest.js';
import { formatStatus } from '../src/status.js';

test('normalizes finding codes', () => {
  assert.equal(normalizeFindingCode('s1'), 'S001');
  assert.equal(normalizeFindingCode(' S012 '), 'S012');
});

test('explains known finding codes', () => {
  const explanation = explainFinding('S001');

  assert.equal(explanation.found, true);
  assert.equal(explanation.code, 'S001');
  assert.match(explanation.summary, /authorization/i);
});

test('formats unknown finding codes with a useful hint', () => {
  assert.match(formatFindingExplanation('S999'), /Unknown finding code/);
});

test('formats latest findings payloads', () => {
  const output = formatLatestFindings({
    state: 'ok',
    source: 'https://sanctifier.example/reports/latest',
    findings: [
      {
        code: 'S003',
        severity: 'high',
        title: 'Unchecked subtraction',
        location: 'src/lib.rs:42',
      },
    ],
  });

  assert.match(output, /S003/);
  assert.match(output, /src\/lib\.rs:42/);
});

test('formats status states', () => {
  assert.match(formatStatus({ state: 'online', source: 'https://sanctifier.example' }), /reachable/);
  assert.match(formatStatus({ state: 'not_configured', source: '' }), /SANCTIFIER_API_URL/);
});
