# Frontend

The web interface for interacting with the Sanctifier suite.

## Tech Stack
- **Framework**: Next.js 14 (App Router)
- **Styling**: Tailwind CSS
- **Wallet Connection**: Freighter (via Stellar Wallets Kit)

## Getting Started

1. Install dependencies:
   ```bash
   npm install
   ```
2. Run development server:
   ```bash
   npm run dev
   ```
3. Open [http://localhost:3000](http://localhost:3000)

## Features
- Upload WASM files for analysis.
- View real-time security reports.
- Dashboard for tracked contracts.

## Behavior Notes
- [Report export (PDF/CSV/JSON)](docs/report-export.md)
- [Offline and dev mode](docs/offline-dev-mode.md)
- [Self-hosting guide](docs/self-hosting.md)
