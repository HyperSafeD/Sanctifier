# Sanctifier Stellar Laboratory Plugin Scaffold

This directory contains a minimal Stellar Laboratory plugin scaffold for Sanctifier.

## What it does

- Reads pasted Soroban contract source
- Sends the source to a hosted Sanctifier analysis API
- Displays JSON results inside the browser

## Files

- `plugin.json` — plugin metadata scaffold
- `index.html` — plugin UI for source entry and result display
- `plugin.js` — hosted API integration and result rendering
- `demo.gif` — recording of the expected plugin flow

## Usage

1. Open `integrations/stellar-lab/index.html` in a compatible Stellar Lab plugin host.
2. Paste Soroban source into the textarea.
3. Click `Analyze with Sanctifier API`.
4. The plugin posts the source to the hosted API and renders the response.

## Hosted API

The plugin currently targets the hosted endpoint defined in `plugin.js`:

```js
const API_URL = "https://api.sanctifier.dev/analyze";
```

Update this URL if your hosted Sanctifier API lives elsewhere.

## Notes

This scaffold is intentionally minimal and designed to be adapted to Stellar Laboratory's external plugin SDK. The code is ready to be wired into the plugin host and extended with authentication, rich result rendering, and branding.
