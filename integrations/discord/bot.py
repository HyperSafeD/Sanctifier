#!/usr/bin/env python3
"""
Sanctifier Discord Bot
Provides slash commands for querying Soroban security findings.
"""

import os
import discord
from discord import app_commands
from discord.ext import commands

# Bot configuration
DISCORD_TOKEN = os.getenv("DISCORD_BOT_TOKEN")
GUILD_ID = os.getenv("DISCORD_GUILD_ID")  # Optional: for faster command sync

# Mock findings database (replace with actual API/DB calls)
FINDINGS_DB = {
    "S001": {
        "id": "S001",
        "title": "Unchecked Arithmetic",
        "severity": "HIGH",
        "description": "Integer overflow/underflow without checked arithmetic",
        "mitigation": "Use checked_add, checked_sub, checked_mul, or checked_div",
    },
    "S002": {
        "id": "S002",
        "title": "Missing Authorization Check",
        "severity": "CRITICAL",
        "description": "Function lacks proper authorization validation",
        "mitigation": "Add require_auth() or equivalent checks",
    },
    "S003": {
        "id": "S003",
        "title": "Reentrancy Risk",
        "severity": "HIGH",
        "description": "External call before state update",
        "mitigation": "Follow checks-effects-interactions pattern",
    },
}

LATEST_FINDINGS = ["S003", "S002", "S001"]


class SanctifierBot(commands.Bot):
    def __init__(self):
        intents = discord.Intents.default()
        intents.message_content = True
        super().__init__(command_prefix="!", intents=intents)

    async def setup_hook(self):
        """Called when the bot is starting up"""
        if GUILD_ID:
            guild = discord.Object(id=int(GUILD_ID))
            self.tree.copy_global_to(guild=guild)
            await self.tree.sync(guild=guild)
        else:
            await self.tree.sync()
        print(f"Synced commands for {self.user}")


bot = SanctifierBot()


@bot.event
async def on_ready():
    print(f"Logged in as {bot.user} (ID: {bot.user.id})")
    print("------")


@bot.tree.command(name="explain", description="Explain a Sanctifier finding by ID")
@app_commands.describe(finding_id="The finding ID (e.g., S001)")
async def explain(interaction: discord.Interaction, finding_id: str):
    """Explain a specific finding"""
    finding_id = finding_id.upper()

    if finding_id not in FINDINGS_DB:
        await interaction.response.send_message(
            f"❌ Finding `{finding_id}` not found. Try `/latest` to see recent findings.",
            ephemeral=True,
        )
        return

    finding = FINDINGS_DB[finding_id]
    embed = discord.Embed(
        title=f"{finding['id']}: {finding['title']}",
        description=finding["description"],
        color=_severity_color(finding["severity"]),
    )
    embed.add_field(name="Severity", value=finding["severity"], inline=True)
    embed.add_field(name="Mitigation", value=finding["mitigation"], inline=False)
    embed.set_footer(text="Sanctifier Security Analysis")

    await interaction.response.send_message(embed=embed)


@bot.tree.command(name="latest", description="Show the latest Sanctifier findings")
async def latest(interaction: discord.Interaction):
    """Show recent findings"""
    embed = discord.Embed(
        title="Latest Sanctifier Findings",
        description="Most recent security findings from Sanctifier",
        color=discord.Color.blue(),
    )

    for finding_id in LATEST_FINDINGS[:5]:
        finding = FINDINGS_DB.get(finding_id)
        if finding:
            embed.add_field(
                name=f"{finding['id']} - {finding['title']}",
                value=f"**{finding['severity']}**: {finding['description'][:100]}...",
                inline=False,
            )

    embed.set_footer(text="Use /explain <ID> for details")
    await interaction.response.send_message(embed=embed)


@bot.tree.command(name="status", description="Check Sanctifier bot status")
async def status(interaction: discord.Interaction):
    """Show bot status"""
    embed = discord.Embed(
        title="Sanctifier Bot Status",
        description="✅ Bot is online and operational",
        color=discord.Color.green(),
    )
    embed.add_field(name="Findings Database", value=f"{len(FINDINGS_DB)} rules", inline=True)
    embed.add_field(name="Latency", value=f"{round(bot.latency * 1000)}ms", inline=True)
    embed.set_footer(text="Sanctifier v1.0.0")

    await interaction.response.send_message(embed=embed)


def _severity_color(severity: str) -> discord.Color:
    """Map severity to Discord embed color"""
    colors = {
        "CRITICAL": discord.Color.red(),
        "HIGH": discord.Color.orange(),
        "MEDIUM": discord.Color.gold(),
        "LOW": discord.Color.blue(),
        "INFO": discord.Color.light_grey(),
    }
    return colors.get(severity, discord.Color.default())


def main():
    if not DISCORD_TOKEN:
        print("Error: DISCORD_BOT_TOKEN environment variable not set")
        print("Set it with: export DISCORD_BOT_TOKEN='your-token-here'")
        return

    bot.run(DISCORD_TOKEN)


if __name__ == "__main__":
    main()
