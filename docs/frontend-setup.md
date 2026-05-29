# Frontend Setup

## Environment variables

Copy `frontend/.env.local.example` to `frontend/.env.local` and set the values before starting the dev server.

```
cp frontend/.env.local.example frontend/.env.local
```

The validated variables and their startup status are printed to the console on first request — look for the `[sanctifier] environment:` table.

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `SANCTIFIER_BIN` | No | `sanctifier` | Path to the sanctifier binary |
| `AI_EXPLAIN_PROVIDER` | Yes (for AI) | — | `anthropic` or `openai` |
| `ANTHROPIC_API_KEY` | Conditional | — | Required when provider is `anthropic` |
| `OPENAI_API_KEY` | Conditional | — | Required when provider is `openai` |
| `RATE_LIMIT_REQUESTS_PER_MINUTE` | No | `10` | Per-IP rate limit on `/api/analyze` |

To run without an AI provider, set `STUB_AI=1` — the explain endpoint will return canned responses.

## Batch contract upload

The dashboard accepts multiple `.rs` files at once via drag-and-drop or the **Upload Contract** file picker (hold Shift/Cmd to select multiple files).

- Each valid file is analyzed independently and added as a workspace member.
- A per-file progress list is shown while analysis is running.
- Files that are not `.rs` or exceed the 250 KB size limit are rejected with an inline notice listing the reason for each file.
- The existing single-file path is unchanged.

## Theme preference

The theme toggle cycles through three modes:

| Mode | Behaviour |
|------|-----------|
| **Light** | Forces light theme regardless of OS setting |
| **Dark** | Forces dark theme regardless of OS setting |
| **System** | Follows the OS `prefers-color-scheme` setting and updates in real time |

The selected mode is stored in `localStorage` under the key `"theme"`. An inline bootstrap script in `layout.tsx` applies the correct class before the first paint to prevent a flash of the wrong theme on page load.
