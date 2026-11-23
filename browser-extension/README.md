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
  A Chrome/Firefox MV3 extension that provides a quick preview of recent Sanctifier security scans without opening the full dashboard.

## Features

- **Quick preview** — Shows the last 10 findings from your local Sanctifier dashboard
- **Background polling** — Periodically fetches latest scan results every 5 minutes
- **Deep linking** — Click any finding to open the full dashboard
- **Severity-aware** — Findings color-coded by severity (critical, high, medium, low)

## Setup

### 1. Build icons

```bash
# Install ImageMagick or librsvg, then:
rsvg-convert -w 16 -h 16 icon.svg > icon-16.png
rsvg-convert -w 48 -h 48 icon.svg > icon-48.png
rsvg-convert -w 128 -h 128 icon.svg > icon-128.png
```

Or generate placeholder PNGs using any image editor.

### 2. Load in Chrome

1. Open `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select the `browser-extension/` directory

### 3. Load in Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the `manifest.json` file

## Usage

**Important:** The extension requires the Sanctifier dashboard to be running on the default ports:

- Dashboard: `http://localhost:3000` (or `http://127.0.0.1:9100`)

If you're running the dashboard on a different port (e.g., because port 3000 is already in use), the extension will not be able to connect due to Manifest V3's strict host permissions. To use a custom port, you'll need to:

1. Edit `manifest.json` and add your custom port to `host_permissions`:
   ```json
   "host_permissions": [
     "http://localhost:3000/*",
     "http://127.0.0.1:9100/*",
     "http://localhost:YOUR_PORT/*"
   ]
   ```
2. Update `DASHBOARD_URL` in `background.js` and `popup.js` to match your port
3. Reload the extension in your browser

### Steps to use:

1. Ensure the Sanctifier dashboard is running at `http://localhost:3000`
2. Click the Sanctifier icon in the browser toolbar
3. View the last 10 findings from recent scans
4. Click a finding or "Open Dashboard" to navigate to the full UI

## Configuration

- **Dashboard URL**: `http://localhost:3000` (default)
- **Poll interval**: 5 minutes (background) / 30 seconds (popup)
- **Max findings displayed**: 10

## Architecture

```
background.js  ←── Alarms (every 5 min) ──→ /api/recent-findings
     │
     └── chrome.storage.local (cached findings)
              │
              ↓
popup.html ──→ popup.js (reads cache + live fetch)
     │
     └── Click → chrome.tabs.create dashboard deep link
```
