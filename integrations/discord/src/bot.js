import { Client, Events, GatewayIntentBits } from 'discord.js';
import { handleInteraction } from './commands.js';

const token = process.env.DISCORD_TOKEN;

if (!token) {
  throw new Error('DISCORD_TOKEN is required to start the Sanctifier Discord bot.');
}

const client = new Client({
  intents: [GatewayIntentBits.Guilds],
});

client.once(Events.ClientReady, (readyClient) => {
  console.log(`Sanctifier Discord bot logged in as ${readyClient.user.tag}`);
});

client.on(Events.InteractionCreate, async (interaction) => {
  try {
    await handleInteraction(interaction);
  } catch (error) {
    console.error('Failed to handle Discord interaction:', error);

    const message = 'Sanctifier could not complete that command. Please try again later.';

    if (interaction.isRepliable()) {
      if (interaction.deferred || interaction.replied) {
        await interaction.editReply(message);
      } else {
        await interaction.reply({ content: message, ephemeral: true });
      }
    }
  }
});

await client.login(token);
