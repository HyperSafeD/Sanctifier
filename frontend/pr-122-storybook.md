# Add UI Component Documentation with Storybook

Closes #122

## Summary

Set up [Storybook](https://storybook.js.org/) to catalog and document all reusable React components in the Sanctifier frontend. Every component now has a dedicated `.stories.tsx` file with multiple usage variants and auto-generated docs.

## What changed

| Area              | Files                                                                                                                                                                                            |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Storybook config  | `.storybook/main.ts`, `.storybook/preview.ts`                                                                                                                                                    |
| Package manifest  | `package.json` — added Storybook scripts and devDependencies                                                                                                                                     |
| Component stories | `CallGraph.stories.tsx`, `CodeSnippet.stories.tsx`, `FindingsList.stories.tsx`, `SanctityScore.stories.tsx`, `SeverityFilter.stories.tsx`, `SummaryChart.stories.tsx`, `ThemeToggle.stories.tsx` |

## Components documented

| Component          | Stories | Key variants                                                    |
| ------------------ | ------- | --------------------------------------------------------------- |
| **CallGraph**      | 4       | Default graph, empty state, functions-only, with severity rings |
| **CodeSnippet**    | 4       | Plain code, highlighted line, single line, empty                |
| **FindingsList**   | 4       | All severities, critical-only filter, no results, empty list    |
| **SanctityScore**  | 5       | Perfect (A), Grade A/B/C, Grade F                               |
| **SeverityFilter** | 3       | All selected, critical selected, medium selected                |
| **SummaryChart**   | 4       | Balanced distribution, critical-heavy, no findings, low-only    |
| **ThemeToggle**    | 1       | Default (interactive toggle)                                    |

## How to run

```bash
cd frontend
npm install
npm run storybook
```

Storybook will be available at `http://localhost:6006`.

## How to build (static export)

```bash
npm run build-storybook
```

Output is written to `frontend/storybook-static/`.

## Proof of successful build

<!-- Attach a screenshot of the Storybook UI running locally or the build output here -->

![Storybook build proof](<!-- PASTE YOUR IMAGE URL OR DRAG-AND-DROP A SCREENSHOT HERE -->)

> **How to get this attachment:**
>
> 1. Run `cd frontend && npm install && npm run storybook`
> 2. Open `http://localhost:6006` in your browser
> 3. Take a screenshot of the Storybook sidebar showing all seven components
> 4. Drag-and-drop the screenshot into this text area when editing the PR on GitHub — GitHub will auto-upload it and fill in the URL
>
> Alternatively, run `npm run build-storybook` and screenshot the successful terminal output.

## Testing

This is a documentation-only change. No production code was modified. To verify:

1. `npm run storybook` launches without errors
2. All seven components render correctly in the Storybook UI
3. Auto-generated docs pages display prop tables and descriptions
4. `npm run build-storybook` exits with code 0
