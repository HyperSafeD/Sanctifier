# Sanctifier Browser Extension

Browser extension for displaying recent security findings from the Sanctifier dashboard.

## Files

- `popup.html` — Extension popup UI
- `popup.js` — Renders the last 10 findings fetched from chrome.storage
- `tests/popup.test.js` — Jest tests covering empty findings and malformed finding shapes (#1893)

## Testing

```bash
cd browser-extension
npm install
npm test
```

## Test Coverage

The test suite (#1893) covers:
- Empty findings array renders "No findings available" message
- Non-array findings (null, undefined, string) render empty state
- Valid findings display with severity, code, message, and location
- Malformed findings with missing fields use fallback values:
  - Missing `severity` → "unknown"
  - Missing `code` → "N/A"
  - Missing `message` → "No description available"
  - Missing `location` → "Unknown location"
- Only first 10 findings are rendered when more than 10 are provided
- HTML in messages is escaped to prevent XSS attacks
