import { SlashCommandBuilder } from 'discord.js';
import { formatFindingExplanation } from './findings.js';
import { fetchLatestFindings, formatLatestFindings } from './latest.js';
import { fetchStatus, formatStatus } from './status.js';

export const commandData = [
  new SlashCommandBuilder()
    .setName('sanctifier')
    .setDescription('Query Sanctifier findings and service status.')
    .addSubcommand((subcommand) =>
      subcommand
        .setName('explain')
        .setDescription('Explain a Sanctifier finding code.')
        .addStringOption((option) =>
          option
            .setName('code')
            .setDescription('Finding code, such as S001.')
            .setRequired(true),
        ),
    )
    .addSubcommand((subcommand) =>
      subcommand
        .setName('latest')
        .setDescription('Show the latest Sanctifier findings from the configured API.')
        .addIntegerOption((option) =>
          option
            .setName('limit')
            .setDescription('Maximum findings to show.')
            .setMinValue(1)
            .setMaxValue(10),
        ),
    )
    .addSubcommand((subcommand) =>
      subcommand
        .setName('status')
        .setDescription('Check whether the configured Sanctifier endpoint is reachable.'),
    ),
].map((command) => command.toJSON());

export async function handleInteraction(interaction) {
  if (!interaction.isChatInputCommand()) return;
  if (interaction.commandName !== 'sanctifier') return;

  const subcommand = interaction.options.getSubcommand();

  if (subcommand === 'explain') {
    const code = interaction.options.getString('code', true);
    await interaction.reply({
      content: formatFindingExplanation(code),
      ephemeral: true,
    });
    return;
  }

  if (subcommand === 'latest') {
    await interaction.deferReply({ ephemeral: true });
    const limit = interaction.options.getInteger('limit') || 5;
    const latest = await fetchLatestFindings();
    await interaction.editReply(formatLatestFindings(latest, limit));
    return;
  }

  if (subcommand === 'status') {
    await interaction.deferReply({ ephemeral: true });
    const status = await fetchStatus();
    await interaction.editReply(formatStatus(status));
  }
}
