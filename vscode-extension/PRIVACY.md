# Sanctifier VS Code Extension — Privacy Policy

## Overview

The Sanctifier VS Code extension may collect anonymous usage telemetry to help us understand how developers use the extension and prioritize improvements.

**Telemetry is opt-in only** and is disabled by default. No data is sent unless you explicitly enable it.

## What we collect

When telemetry is enabled, the extension sends the following anonymous data:

- **Scan count**: The number of times the analyzer has run
- **Finding count by rule**: The number of findings per rule code (e.g., `S001: 5`, `S002: 3`)
- **Extension version**: The version of the Sanctifier extension you are using
- **VS Code version**: The version of VS Code you are using

## What we NEVER collect

We do not and will never collect:

- Source code or file contents
- File paths or workspace names
- Contract names or addresses
- Repository URLs
- Personal information (names, emails, IP addresses)
- Environment variables or secrets
- Any other identifiable information

## How we use the data

The collected data helps us:

- Understand which rules generate the most findings
- Prioritize bug fixes and feature development
- Understand VS Code version distribution for compatibility testing

## Data storage and transmission

- Data is sent via HTTPS to our analytics endpoint
- No third-party analytics services are used
- Data is aggregated and anonymized
- No individual user or session identification is tracked

## How to enable or disable telemetry

You can enable or disable telemetry at any time in VS Code settings:

1. Open Settings (`Cmd+,` or `Ctrl+,`)
2. Search for `sanctifier.telemetry.enabled`
3. Toggle the setting

Or in `settings.json`:

```json
{
  "sanctifier.telemetry.enabled": true
}
```

When you first install the extension, you will be prompted to opt in. If you decline, no telemetry data will be sent.

## Changes to this policy

We may update this privacy policy from time to time. Changes will be reflected in this file and noted in the extension's changelog.

## Contact

For questions about this privacy policy, please open an issue at:
https://github.com/HyperSafeD/Sanctifier/issues
