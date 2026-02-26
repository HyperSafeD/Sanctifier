# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities in Sanctifier seriously. If you discover a security issue, please follow the responsible disclosure process outlined below.

### How to Report

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Send an email to **security@sanctifier.dev** with the following information:
   - A clear description of the vulnerability
   - Steps to reproduce the issue
   - The affected component (CLI, core library, frontend, smart contracts)
   - The potential impact and severity assessment
   - Any suggested fix or mitigation (optional)

### What to Expect

- **Acknowledgement**: You will receive an acknowledgement within **48 hours** of your report.
- **Assessment**: We will assess the vulnerability and determine its severity within **5 business days**.
- **Resolution**: We aim to release a fix within **30 days** of confirming the vulnerability, depending on complexity.
- **Disclosure**: We will coordinate with you on public disclosure timing. We request a **90-day disclosure window** from the initial report.

### Severity Levels

| Level    | Description                                              | Response Time |
|----------|----------------------------------------------------------|---------------|
| Critical | Remote code execution, data loss, authentication bypass  | 24 hours      |
| High     | Significant impact on analysis accuracy or data integrity| 3 days        |
| Medium   | Limited impact, requires specific conditions             | 7 days        |
| Low      | Minimal impact, informational                            | 30 days       |

### Scope

The following components are in scope for vulnerability reports:

- **sanctifier-core**: Static analysis engine
- **sanctifier-cli**: Command-line interface
- **Frontend dashboard**: Web-based visualization
- **Smart contract examples**: Only if vulnerabilities affect the analysis tooling itself

### Out of Scope

- Vulnerabilities in third-party dependencies (report these to the respective maintainers)
- Issues in the example vulnerable contracts (these are intentionally insecure for demonstration)
- Denial of service through excessively large input files

## Safe Harbor

We consider security research conducted in accordance with this policy to be:

- Authorized and welcome
- Conducted in good faith
- Not subject to legal action from our side

We will not pursue legal action against researchers who follow this responsible disclosure process.

## Recognition

We appreciate the security research community's efforts. With your permission, we will acknowledge your contribution in our release notes and security advisories.
