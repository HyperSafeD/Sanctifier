# Sanctifier Discord Bot

A Discord bot that provides slash commands for querying Soroban security findings from Sanctifier.

## Features

- `/explain <finding_id>` - Get detailed explanation of a specific finding (e.g., `/explain S001`)
- `/latest` - View the most recent security findings
- `/status` - Check bot health and statistics

## Setup

### 1. Create a Discord Bot

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" tab and click "Add Bot"
4. Under "Privileged Gateway Intents", enable:
   - Message Content Intent (if you want to read messages)
5. Copy the bot token (you'll need this later)

### 2. Invite Bot to Your Server

1. Go to the "OAuth2" → "URL Generator" tab
2. Select scopes:
   - `bot`
   - `applications.commands`
3. Select bot permissions:
   - Send Messages
   - Embed Links
   - Use Slash Commands
4. Copy the generated URL and open it in your browser to invite the bot

### 3. Install Dependencies

```bash
cd integrations/discord
pip install -r requirements.txt
```

Or using a virtual environment:

```bash
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
```

### 4. Configure Environment Variables

Create a `.env` file in the `integrations/discord/` directory:

```env
DISCORD_BOT_TOKEN=your_bot_token_here
DISCORD_GUILD_ID=your_guild_id_here  # Optional: for faster command sync
```

Or export them directly:

```bash
export DISCORD_BOT_TOKEN='your_bot_token_here'
export DISCORD_GUILD_ID='your_guild_id_here'  # Optional
```

### 5. Run the Bot

```bash
python bot.py
```

You should see:
```
Synced commands for SanctifierBot#1234
Logged in as SanctifierBot (ID: 123456789)
------
```

## Usage

In any Discord channel where the bot has access, use:

- `/explain S001` - Get details about finding S001
- `/latest` - See the 5 most recent findings
- `/status` - Check if the bot is online

## Deployment

### Docker (Recommended)

Create a `Dockerfile`:

```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY bot.py .

CMD ["python", "bot.py"]
```

Build and run:

```bash
docker build -t sanctifier-bot .
docker run -e DISCORD_BOT_TOKEN='your_token' sanctifier-bot
```

### systemd Service

Create `/etc/systemd/system/sanctifier-bot.service`:

```ini
[Unit]
Description=Sanctifier Discord Bot
After=network.target

[Service]
Type=simple
User=sanctifier
WorkingDirectory=/opt/sanctifier/integrations/discord
Environment="DISCORD_BOT_TOKEN=your_token_here"
ExecStart=/usr/bin/python3 /opt/sanctifier/integrations/discord/bot.py
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable sanctifier-bot
sudo systemctl start sanctifier-bot
sudo systemctl status sanctifier-bot
```

## Customization

### Connect to Real Database

Replace the `FINDINGS_DB` mock dictionary in `bot.py` with actual API calls:

```python
import requests

def get_finding(finding_id: str):
    response = requests.get(f"https://api.sanctifier.io/findings/{finding_id}")
    return response.json()
```

### Add More Commands

```python
@bot.tree.command(name="search", description="Search findings by keyword")
@app_commands.describe(keyword="Search term")
async def search(interaction: discord.Interaction, keyword: str):
    # Your search logic here
    await interaction.response.send_message(f"Searching for: {keyword}")
```

## Troubleshooting

**Bot doesn't respond to commands:**
- Make sure slash commands are synced (check bot startup logs)
- Verify the bot has "Use Application Commands" permission
- Try kicking and re-inviting the bot

**"Interaction failed" error:**
- Ensure the bot responds within 3 seconds
- Use `await interaction.response.defer()` for long operations

**Commands not showing up:**
- Global commands can take up to 1 hour to sync
- Use `DISCORD_GUILD_ID` for instant sync during development

## License

MIT License - see main Sanctifier repository for details.
