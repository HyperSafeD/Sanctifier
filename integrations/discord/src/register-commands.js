import { REST, Routes } from 'discord.js';
import { commandData } from './commands.js';

const token = process.env.DISCORD_TOKEN;
const clientId = process.env.DISCORD_CLIENT_ID;
const guildId = process.env.DISCORD_GUILD_ID;

if (!token) {
  throw new Error('DISCORD_TOKEN is required to register slash commands.');
}

if (!clientId) {
  throw new Error('DISCORD_CLIENT_ID is required to register slash commands.');
}

const rest = new REST({ version: '10' }).setToken(token);
const route = guildId
  ? Routes.applicationGuildCommands(clientId, guildId)
  : Routes.applicationCommands(clientId);

await rest.put(route, { body: commandData });

console.log(
  `Registered ${commandData.length} Sanctifier Discord command(s)` +
    (guildId ? ` for guild ${guildId}.` : ' globally.'),
);
