/**
 * Tests for browser-extension/popup.js (#1893).
 *
 * Covers empty findings and malformed finding shapes to ensure the popup
 * renders gracefully without breaking when data is missing or invalid.
 */

const { JSDOM } = require('jsdom');
const fs = require('fs');
const path = require('path');

// Load popup.html and popup.js
const html = fs.readFileSync(path.join(__dirname, '../popup.html'), 'utf8');
const popupScript = fs.readFileSync(path.join(__dirname, '../popup.js'), 'utf8');

describe('popup.js rendering', () => {
  let dom;
  let document;
  let window;
  let renderFindings;

  beforeEach(() => {
    // Create a fresh DOM for each test
    dom = new JSDOM(html, { runScripts: 'outside-only' });
    document = dom.window.document;
    window = dom.window;

    // Execute popup.js in the JSDOM context
    dom.window.eval(popupScript);
    renderFindings = dom.window.renderFindings;
  });

  afterEach(() => {
    dom.window.close();
  });

  test('renders empty state when findings array is empty', () => {
    renderFindings([]);

    const container = document.getElementById('findings-container');
    expect(container.textContent).toContain('No findings available');
    expect(container.querySelector('.findings-list')).toBeNull();
  });

  test('renders empty state when findings is not an array', () => {
    renderFindings(null);
    let container = document.getElementById('findings-container');
    expect(container.textContent).toContain('No findings available');

    renderFindings(undefined);
    container = document.getElementById('findings-container');
    expect(container.textContent).toContain('No findings available');

    renderFindings('not an array');
    container = document.getElementById('findings-container');
    expect(container.textContent).toContain('No findings available');
  });

  test('renders findings with all fields present', () => {
    const findings = [
      {
        severity: 'error',
        code: 'S001',
        message: 'Authentication gap detected',
        location: './contracts/token/src/lib.rs:42',
      },
      {
        severity: 'warning',
        code: 'S003',
        message: 'Unchecked arithmetic operation',
        location: './contracts/token/src/lib.rs:30',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const items = container.querySelectorAll('.finding-item');

    expect(items.length).toBe(2);

    expect(items[0].textContent).toContain('ERROR');
    expect(items[0].textContent).toContain('S001');
    expect(items[0].textContent).toContain('Authentication gap detected');
    expect(items[0].textContent).toContain('./contracts/token/src/lib.rs:42');

    expect(items[1].textContent).toContain('WARNING');
    expect(items[1].textContent).toContain('S003');
  });

  test('handles malformed finding with missing severity field', () => {
    const findings = [
      {
        code: 'S009',
        message: 'Unhandled result',
        location: './contracts/foo/src/lib.rs:10',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const item = container.querySelector('.finding-item');

    expect(item).not.toBeNull();
    expect(item.textContent).toContain('UNKNOWN'); // Fallback severity
    expect(item.textContent).toContain('S009');
    expect(item.textContent).toContain('Unhandled result');
  });

  test('handles malformed finding with missing code field', () => {
    const findings = [
      {
        severity: 'info',
        message: 'Some info message',
        location: './contracts/bar/src/lib.rs:5',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const item = container.querySelector('.finding-item');

    expect(item).not.toBeNull();
    expect(item.textContent).toContain('INFO');
    expect(item.textContent).toContain('N/A'); // Fallback code
    expect(item.textContent).toContain('Some info message');
  });

  test('handles malformed finding with missing message field', () => {
    const findings = [
      {
        severity: 'error',
        code: 'S012',
        location: './contracts/baz/src/lib.rs:20',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const item = container.querySelector('.finding-item');

    expect(item).not.toBeNull();
    expect(item.textContent).toContain('ERROR');
    expect(item.textContent).toContain('S012');
    expect(item.textContent).toContain('No description available'); // Fallback message
  });

  test('handles malformed finding with missing location field', () => {
    const findings = [
      {
        severity: 'warning',
        code: 'S005',
        message: 'Potential reentrancy',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const item = container.querySelector('.finding-item');

    expect(item).not.toBeNull();
    expect(item.textContent).toContain('WARNING');
    expect(item.textContent).toContain('S005');
    expect(item.textContent).toContain('Potential reentrancy');
    expect(item.textContent).toContain('Unknown location'); // Fallback location
  });

  test('renders only the first 10 findings when more than 10 are provided', () => {
    const findings = Array.from({ length: 15 }, (_, i) => ({
      severity: 'info',
      code: `S${i + 1}`,
      message: `Finding ${i + 1}`,
      location: `./contracts/test/src/lib.rs:${i + 1}`,
    }));

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const items = container.querySelectorAll('.finding-item');

    expect(items.length).toBe(10);
  });

  test('escapes HTML in message to prevent XSS', () => {
    const findings = [
      {
        severity: 'error',
        code: 'S001',
        message: '<script>alert("xss")</script>',
        location: './contracts/evil/src/lib.rs:1',
      },
    ];

    renderFindings(findings);

    const container = document.getElementById('findings-container');
    const messageEl = container.querySelector('.message');

    // The script tag should be escaped, not executed
    expect(messageEl.textContent).toBe('<script>alert("xss")</script>');
    expect(messageEl.innerHTML).not.toContain('<script>');
    expect(container.querySelector('script')).toBeNull();
  });
});
