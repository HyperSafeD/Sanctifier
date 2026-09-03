import {
  findingsToSarif,
  sarifToFindings,
  serialiseSarif,
  parseSarif,
  validateSarifShape,
  SARIF_VERSION,
  SARIF_SCHEMA,
} from './sarif';
import type { EditorFinding } from './analyzer';

const SAMPLE_FINDINGS: EditorFinding[] = [
  { line: 10, code: 'S001', severity: 'warning', message: 'Missing require_auth' },
  { line: 20, code: 'S002', severity: 'error', message: 'panic! used', endLine: 20 },
  { line: 30, code: 'S003', severity: 'warning', message: 'Unchecked arithmetic' },
];

describe('findingsToSarif', () => {
  it('produces a SARIF 2.1.0 log with the correct version', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    expect(log.version).toBe(SARIF_VERSION);
    expect(log.$schema).toBe(SARIF_SCHEMA);
  });

  it('includes one run with all results', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    expect(log.runs).toHaveLength(1);
    expect(log.runs[0].results).toHaveLength(SAMPLE_FINDINGS.length);
  });

  it('maps severity correctly: error → error, warning → warning, information → note', () => {
    const findings: EditorFinding[] = [
      { line: 1, code: 'S002', severity: 'error', message: 'e' },
      { line: 2, code: 'S001', severity: 'warning', message: 'w' },
      { line: 3, code: 'S001', severity: 'information', message: 'i' },
    ];
    const log = findingsToSarif(findings, 'f.rs');
    const levels = log.runs[0].results.map((r) => r.level);
    expect(levels).toEqual(['error', 'warning', 'note']);
  });

  it('sets startLine from finding.line', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    expect(log.runs[0].results[0].locations[0].physicalLocation.region.startLine).toBe(10);
  });

  it('includes endLine when finding has endLine', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    const panicResult = log.runs[0].results.find((r) => r.ruleId === 'S002');
    expect(panicResult?.locations[0].physicalLocation.region.endLine).toBe(20);
  });

  it('de-duplicates rules — each ruleId appears only once', () => {
    const findings: EditorFinding[] = [
      { line: 1, code: 'S001', severity: 'warning', message: 'a' },
      { line: 2, code: 'S001', severity: 'warning', message: 'b' },
    ];
    const log = findingsToSarif(findings, 'f.rs');
    expect(log.runs[0].tool.driver.rules).toHaveLength(1);
  });

  it('produces valid JSON via serialiseSarif', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    const json = serialiseSarif(log);
    expect(() => JSON.parse(json)).not.toThrow();
  });
});

describe('sarifToFindings', () => {
  it('round-trips findings through SARIF', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    const recovered = sarifToFindings(log);
    expect(recovered).toHaveLength(SAMPLE_FINDINGS.length);
    expect(recovered[0].line).toBe(SAMPLE_FINDINGS[0].line);
    expect(recovered[0].code).toBe(SAMPLE_FINDINGS[0].code);
    expect(recovered[0].severity).toBe(SAMPLE_FINDINGS[0].severity);
    expect(recovered[0].message).toBe(SAMPLE_FINDINGS[0].message);
  });

  it('maps note level back to information severity', () => {
    const finding: EditorFinding = { line: 5, code: 'S001', severity: 'information', message: 'info' };
    const log = findingsToSarif([finding], 'f.rs');
    const recovered = sarifToFindings(log);
    expect(recovered[0].severity).toBe('information');
  });

  it('returns empty array for a log with no results', () => {
    const log = findingsToSarif([], 'f.rs');
    expect(sarifToFindings(log)).toHaveLength(0);
  });
});

describe('parseSarif / validateSarifShape', () => {
  it('validateSarifShape accepts a well-formed log', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    expect(validateSarifShape(log)).toBe(true);
  });

  it('validateSarifShape rejects wrong version', () => {
    const bad = { version: '1.0.0', runs: [] };
    expect(validateSarifShape(bad)).toBe(false);
  });

  it('validateSarifShape rejects null', () => {
    expect(validateSarifShape(null)).toBe(false);
  });

  it('validateSarifShape rejects missing runs', () => {
    expect(validateSarifShape({ version: '2.1.0' })).toBe(false);
  });

  it('parseSarif round-trips the log', () => {
    const log = findingsToSarif(SAMPLE_FINDINGS, 'src/lib.rs');
    const json = serialiseSarif(log);
    const parsed = parseSarif(json);
    expect(parsed.version).toBe(SARIF_VERSION);
    expect(parsed.runs[0].results).toHaveLength(SAMPLE_FINDINGS.length);
  });

  it('parseSarif throws on invalid JSON', () => {
    expect(() => parseSarif('not-json')).toThrow();
  });
});
