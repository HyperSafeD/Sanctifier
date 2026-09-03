import {
  formatLine,
  formatScanStart,
  formatCliExit,
  formatFindings,
  renderOutputBlock,
  truncateOutput,
  PREFIX,
} from './output-format';
import type { OutputLine } from './output-format';

// ---------------------------------------------------------------------------
// formatLine – correctness
// ---------------------------------------------------------------------------

describe('formatLine', () => {
  it('prefixes info lines', () => {
    expect(formatLine({ kind: 'info', text: 'hello' })).toBe(`${PREFIX} hello`);
  });

  it('prefixes stderr lines with [stderr] tag', () => {
    expect(formatLine({ kind: 'stderr', text: 'oops' })).toBe(`${PREFIX}[stderr] oops`);
  });

  it('prefixes error lines with [error] tag', () => {
    expect(formatLine({ kind: 'error', text: 'fatal' })).toBe(`${PREFIX}[error] fatal`);
  });

  it('prefixes debug lines with [debug] tag', () => {
    expect(formatLine({ kind: 'debug', text: 'verbose' })).toBe(`${PREFIX}[debug] verbose`);
  });

  it('passes result JSON through unchanged', () => {
    const json = '{"findings":[]}';
    expect(formatLine({ kind: 'result', json })).toBe(json);
  });

  it('formats done line', () => {
    expect(formatLine({ kind: 'done' })).toBe(`${PREFIX} Scan complete.`);
  });
});

// ---------------------------------------------------------------------------
// Helper formatters
// ---------------------------------------------------------------------------

describe('formatScanStart', () => {
  it('includes the fsPath in the message', () => {
    const line = formatScanStart('/home/user/project');
    expect(line).toContain('/home/user/project');
  });

  it('includes the scanning ellipsis', () => {
    expect(formatScanStart('.')).toMatch(/…$/);
  });

  it('has the expected prefix', () => {
    expect(formatScanStart('.')).toContain(PREFIX);
  });
});

describe('formatCliExit', () => {
  it('includes the exit code', () => {
    expect(formatCliExit(1)).toContain('1');
    expect(formatCliExit(0)).toContain('0');
  });
});

describe('formatFindings', () => {
  it('reports "No findings." when count is 0', () => {
    expect(formatFindings(0)).toContain('No findings');
  });

  it('uses singular "finding" for count 1', () => {
    expect(formatFindings(1)).toContain('1 finding ');
  });

  it('uses plural "findings" for count > 1', () => {
    expect(formatFindings(5)).toContain('5 findings');
  });
});

// ---------------------------------------------------------------------------
// renderOutputBlock
// ---------------------------------------------------------------------------

describe('renderOutputBlock', () => {
  it('joins lines with newline', () => {
    const lines: OutputLine[] = [
      { kind: 'info', text: 'start' },
      { kind: 'done' },
    ];
    const rendered = renderOutputBlock(lines);
    const parts = rendered.split('\n');
    expect(parts).toHaveLength(2);
  });

  it('returns empty string for empty array', () => {
    expect(renderOutputBlock([])).toBe('');
  });

  it('formats each line in order', () => {
    const lines: OutputLine[] = [
      { kind: 'info', text: 'first' },
      { kind: 'stderr', text: 'second' },
      { kind: 'done' },
    ];
    const rendered = renderOutputBlock(lines);
    const parts = rendered.split('\n');
    expect(parts[0]).toContain('first');
    expect(parts[1]).toContain('[stderr]');
    expect(parts[2]).toContain('Scan complete');
  });
});

// ---------------------------------------------------------------------------
// truncateOutput
// ---------------------------------------------------------------------------

describe('truncateOutput', () => {
  it('returns the original string if within budget', () => {
    const text = 'hello world';
    expect(truncateOutput(text, 1024)).toBe(text);
  });

  it('truncates output that exceeds the byte budget', () => {
    const big = 'x'.repeat(300_000);
    const result = truncateOutput(big, 256 * 1024);
    expect(result.length).toBeLessThan(big.length);
  });

  it('includes a truncation notice in the output', () => {
    const big = 'x'.repeat(300_000);
    const result = truncateOutput(big, 256 * 1024);
    expect(result).toContain('output truncated');
  });

  it('truncated output is still non-empty', () => {
    const big = 'z'.repeat(500_000);
    const result = truncateOutput(big, 100);
    expect(result.length).toBeGreaterThan(0);
  });

  it('does not truncate exactly at budget boundary', () => {
    const text = 'a'.repeat(100);
    expect(truncateOutput(text, 100)).toBe(text);
  });
});

// ---------------------------------------------------------------------------
// Performance budgets – output panel formatting
// ---------------------------------------------------------------------------

describe('output panel formatting – performance budgets', () => {
  const BUDGET_MS = 5;
  const LARGE_FINDING_COUNT = 1_000;

  it(`formats ${LARGE_FINDING_COUNT} findings lines in under ${BUDGET_MS}ms`, () => {
    const lines: OutputLine[] = Array.from({ length: LARGE_FINDING_COUNT }, (_, i) => ({
      kind: 'info' as const,
      text: `Finding S00${i % 9 + 1} at contracts/token.rs:${i + 1}`,
    }));

    const start = performance.now();
    renderOutputBlock(lines);
    const elapsed = performance.now() - start;

    expect(elapsed).toBeLessThan(BUDGET_MS);
  });

  it('formats a 10 000-character stderr line in under 1ms', () => {
    const line: OutputLine = { kind: 'stderr', text: 'e'.repeat(10_000) };
    const start = performance.now();
    formatLine(line);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(1);
  });

  it('truncates a 1 MB output string in under 10ms', () => {
    const megabyte = 'x'.repeat(1024 * 1024);
    const start = performance.now();
    truncateOutput(megabyte, 256 * 1024);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(10);
  });

  it('renderOutputBlock with 100 mixed-kind lines stays under 2ms', () => {
    const kinds: OutputLine['kind'][] = ['info', 'stderr', 'error', 'debug', 'done'];
    const lines: OutputLine[] = Array.from({ length: 100 }, (_, i) => {
      const kind = kinds[i % kinds.length];
      if (kind === 'done') return { kind };
      if (kind === 'result') return { kind: 'result', json: '{}' };
      return { kind, text: `message ${i}` } as OutputLine;
    });

    const start = performance.now();
    renderOutputBlock(lines);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(2);
  });

  it('PREFIX constant is present and starts with [', () => {
    expect(PREFIX).toMatch(/^\[/);
  });
});

// ---------------------------------------------------------------------------
// Output format – no leaking newlines
// ---------------------------------------------------------------------------

describe('output format – line discipline', () => {
  it('single formatted line contains no trailing newline', () => {
    const line = formatLine({ kind: 'info', text: 'test' });
    expect(line).not.toMatch(/\n$/);
  });

  it('stderr formatted line does not introduce extra newlines', () => {
    const line = formatLine({ kind: 'stderr', text: 'oops' });
    expect(line.split('\n')).toHaveLength(1);
  });

  it('done line is a single line', () => {
    const line = formatLine({ kind: 'done' });
    expect(line.split('\n')).toHaveLength(1);
  });
});
