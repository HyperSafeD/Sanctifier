# Frontend

The web interface for interacting with the Sanctifier suite.

## Tech Stack
- **Framework**: Next.js (App Router)
- **Styling**: Tailwind CSS
- **State**: React Context + `useReducer` (see [State management](#state-management))
- **Testing**: Vitest + React Testing Library
- **Wallet Connection**: Freighter (via Stellar Wallets Kit)

## Getting Started

Requires **Node.js 20+** and **npm 10+** (enforced by the `engines` field in `package.json`).

1. Install dependencies:
   ```bash
   npm ci        # use `npm install` if you are changing dependencies
   ```
2. Run development server:
   ```bash
   npm run dev
   ```
3. Open [http://localhost:3000](http://localhost:3000)
4. Run the tests:
   ```bash
   npm test
   ```

## State management

Shared state lives in providers under `app/providers/`, not in page-level `useState` calls:

| Provider | Mounted in | Owns |
|----------|-----------|------|
| `ThemeProvider` | `app/layout.tsx` | Light / dark / high-contrast theme |
| `ToastProvider` | `app/layout.tsx` | Transient notifications |
| `WorkspaceProvider` | `app/layout.tsx` | The loaded workspace, selected contract, and their persistence |
| `DashboardProvider` | `app/dashboard/layout.tsx` | Dashboard *view* state — filters, active tab, upload progress, trend records, source cache |

`DashboardProvider` is a `useReducer` store. Two things are worth knowing before you change it:

- **State and actions are separate contexts.** `useDashboardState()` re-renders on every change;
  `useDashboardActions()` returns a referentially stable object, so a component that only
  dispatches never re-renders when an unrelated keystroke lands. That stability is also what lets
  `actions` sit safely in `useCallback` dependency arrays.
- **Multi-field transitions are single actions.** `uploadStarted` / `uploadSucceeded` /
  `uploadFailed` each commit every field they touch at once, so the UI can never observe a
  half-applied upload (an error banner with the spinner still running, say). Prefer adding a named
  action over chaining setters.

The reducer is exported as `dashboardReducer` and unit-tested directly in
`app/providers/DashboardProvider.test.tsx` — transitions can be tested without mounting a tree.

## Features
- Upload WASM files for analysis.
- View real-time security reports.
- Dashboard for tracked contracts.
- **Cookie-free by default** - No tracking, analytics, or non-essential cookies (see [Cookie Audit](docs/COOKIE_AUDIT.md))

## Behavior Notes
- [Report export (PDF/CSV/JSON)](docs/report-export.md)
- [Offline and dev mode](docs/offline-dev-mode.md)
- [Self-hosting guide](docs/self-hosting.md)
- [Cookie & tracking audit](docs/COOKIE_AUDIT.md) - GDPR/ePrivacy compliance documentation
