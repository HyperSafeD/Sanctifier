# Frontend Report Export (PDF/CSV/JSON)

This note documents the current report export behavior in `frontend` and the guardrails contributors should keep when extending it.

## Scope and owner files

- Dashboard orchestration: `frontend/app/dashboard/page.tsx`
- Export controls: `frontend/app/components/DashboardHeader.tsx`
- PDF generation logic: `frontend/app/lib/export-pdf.ts`

## Current behavior

### JSON

- JSON is the source of truth for the dashboard state.
- Users can load JSON by:
  - pasting report JSON into the dashboard text area
  - uploading a `.json` file
  - uploading a Rust contract (`.rs`) and letting `/api/analyze` return findings
- Parsed data is normalized and transformed before rendering.

### PDF

- `Export PDF` is enabled only when findings/callgraph data exists.
- PDF generation is client-side via `jspdf` in `export-pdf.ts`.
- PDF output includes:
  - header metadata (generated timestamp, finding count)
  - score and severity summary
  - grouped findings with category/location/snippet/suggestion
- If `jspdf` import fails, the UI falls back to `window.print()`.

### CSV

- Dedicated CSV export is not implemented yet.
- To avoid format drift, contributors should treat JSON as canonical and derive CSV from normalized findings only.

## Contribution notes

- Keep export behavior deterministic for the same normalized report input.
- Avoid introducing divergent parsing paths between upload/paste/API results.
- Add new exports behind explicit controls in `DashboardHeader` and reuse normalized findings from `page.tsx`.
- Keep generated filename conventions stable unless a migration note is added.
