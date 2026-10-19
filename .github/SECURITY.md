# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a Vulnerability

Please do not open public issues for suspected security vulnerabilities.

Report vulnerabilities by emailing security@sanctifier.dev with:

- A clear description of the issue and affected component
- Steps to reproduce or a proof of concept
- Expected impact and any severity assessment
- Proposed remediation or mitigation, if available

If you prefer GitHub-native reporting, you may also open a private security advisory draft through the repository's Security tab.

## Response Expectations

- We aim to acknowledge reports within 48 hours.
- We aim to complete an initial triage within 5 business days.
- We will coordinate remediation and disclosure with the reporter.
- We request a 90-day responsible disclosure window from the initial report before public disclosure.

## Scope

In scope:

- sanctifier-core
- sanctifier-cli
- frontend dashboard
- GitHub Action and release artifacts
- Supporting analysis data and schemas when they affect security outcomes

Out of scope:

- Vulnerabilities in third-party dependencies without a Sanctifier-specific exploit path
- Intentionally vulnerable example contracts included for demonstration or testing
- Denial of service caused solely by unrealistic resource exhaustion inputs

## Safe Harbor

We support good-faith security research conducted under this policy. We will not pursue legal action against researchers who act in good faith, avoid privacy violations and service disruption, and provide us a reasonable opportunity to investigate and remediate the issue before disclosure.