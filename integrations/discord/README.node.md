# Sanctifier Node Discord Bot

Minimal Discord integration for querying Sanctifier findings from slash commands.

## Commands

| Command | Purpose |
| --- | --- |
| `/sanctifier explain code:S001` | Explains a canonical Sanctifier finding code. |
| `/sanctifier latest limit:5` | Shows recent findings from the configured latest-report endpoint. |
| `/sanctifier status` | Checks whether the configured Sanctifier endpoint is reachable. |

## Setup

1. Create a Discord application and bot at <https://discord.com/developers/applications>.
2. Copy `.env.node.example` to `.env` or set the same variables in your deployment environment.
3. Install dependencies:

   ```bash
   cd integrations/discord
   npm install
   ```

4. Register slash commands:

   ```bash
   npm run register
   ```

   Set `DISCORD_GUILD_ID` while testing so commands appear immediately in one server. Omit it for global registration when deploying.

5. Start the bot:

   ```bash
   npm start
   ```

## Environment

| Variable | Required | Description |
| --- | --- | --- |
| `DISCORD_TOKEN` | Yes | Bot token from the Discord developer portal. |
| `DISCORD_CLIENT_ID` | Yes | Application client ID used when registering slash commands. |
| `DISCORD_GUILD_ID` | No | Guild/server ID for fast test registration. |
| `SANCTIFIER_API_URL` | No | Base URL for a hosted Sanctifier dashboard/API. |
| `SANCTIFIER_LATEST_URL` | No | Full URL returning latest report JSON. Overrides `SANCTIFIER_API_URL` for `/latest`. |
| `SANCTIFIER_STATUS_URL` | No | Full URL used by `/status`. Defaults to `SANCTIFIER_API_URL`. |

The `/latest` command accepts JSON in any of these shapes:

```json
{ "findings": [] }
```

```json
{ "latest": { "findings": [] } }
```

```json
{ "report": { "findings": [] } }
```

## Deployment Notes

- Use Node 20 or newer.
- Run `npm run register` during release or whenever command definitions change.
- Run `npm start` as a long-lived process on Fly.io, Render, a VM, or any worker platform that supports outbound Discord gateway connections.
- Keep Discord secrets in the platform secret store, not in the repository.
- For staging, register to a single guild with `DISCORD_GUILD_ID`; for production, use global registration.
