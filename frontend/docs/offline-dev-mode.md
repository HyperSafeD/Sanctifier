# Frontend Offline and Dev Mode

This note documents how the dashboard behaves when backend analysis is unavailable and how contributors should develop safely in local/offline workflows.

## Scope and owner files

- Analyze API route: `frontend/app/api/analyze/route.ts`
- Dashboard upload/parse flow: `frontend/app/dashboard/page.tsx`
- Upload controls: `frontend/app/components/DashboardHeader.tsx`
- Finding filter logic: `frontend/app/lib/finding-filters.ts`
- Upload validation: `frontend/app/lib/upload-validation.ts`

## Operational modes

### Local JSON-only mode (offline-safe)

- Users can still work fully from local JSON reports without calling backend analysis.
- Uploading or pasting JSON works without the `sanctifier` binary.
- This mode is ideal for UI iteration, triage review, and reproducible demos.

### Contract upload mode (requires local tooling)

- Contract upload posts source to `/api/analyze`.
- The route executes `sanctifier analyze --format json` using:
  - `SANCTIFIER_BIN` if set
  - `sanctifier` in PATH otherwise
- Missing binary, timeout, invalid content type, unsupported extension, invalid UTF-8, and oversized payloads return explicit API errors.
- The dashboard now validates extension and size client-side before upload and surfaces errors immediately.

### Finding-code filtering mode

- The findings panel supports exact finding-code search using canonical format `S###` (example: `S001`).
- Input is normalized to uppercase and validated before filtering.
- Legacy non-canonical codes are normalized to canonical `S` codes at transform time to keep filtering predictable.

## Dev guardrails

- Keep JSON parsing (`parseReport`) independent from runtime analysis so UI workflows remain available during backend outages.
- Preserve input limits and extension checks in the API route.
- Keep error strings user-readable because they are surfaced directly in the dashboard.
- If adding new dev flags or env vars, document defaults and fallback behavior in this file.
