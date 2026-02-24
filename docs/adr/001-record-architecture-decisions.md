# ADR 001: Record Architecture Decisions

## Status

Accepted

## Context

We need to record the architectural decisions made on this project. Without a formalized process, important technical decisions, their context, and the rationale behind them are lost over time. This leads to repeated discussions, inconsistent architecture, and a steep learning curve for new developers joining the Sanctifier project.

## Decision

We will use Architecture Decision Records, as described by Michael Nygard in his article "Documenting Architecture Decisions". 

We will store these records in the `docs/adr/` directory of the project repository. Each record will be written in Markdown to remain lightweight, version-controlled alongside the code, and easily readable on GitHub or in any text editor.

The standard template will include:
- **Title**: A short noun phrase containing the ADR number and topic. (e.g., "ADR 001: Record Architecture Decisions")
- **Status**: What is the status, such as "Proposed", "Accepted", "Rejected", "Deprecated", "Superseded".
- **Context**: What is the issue that we're seeing that is motivating this decision or change.
- **Decision**: What is the change that we're proposing and/or doing.
- **Consequences**: What becomes easier or more difficult to do because of this change.

## Consequences

See Michael Nygard's article, linked above. For a lightweight ADR toolset, see Nat Pryce's `adr-tools` at https://github.com/npryce/adr-tools.

**Positive:**
* A clear, searchable history of architectural decisions.
* Better onboarding for new team members.
* Reduced need to re-litigate past decisions unless the underlying context changes.

**Negative:**
* Requires discipline from developers to document decisions.
* Slight overhead when making significant architectural changes.
