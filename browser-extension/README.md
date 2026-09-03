# Sanctifier Browser Extension

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
