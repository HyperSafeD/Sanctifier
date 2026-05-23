export const FINDINGS = {
  S001: {
    title: 'Missing require_auth',
    severity: 'critical',
    summary: 'A state-changing path can run without a Soroban authorization guard.',
    remediation: 'Require the caller, owner, admin, or affected address to call require_auth before mutating state.',
  },
  S002: {
    title: 'Panic, unwrap, or expect in contract path',
    severity: 'high',
    summary: 'A runtime panic can lock execution or hide recoverable error handling.',
    remediation: 'Return explicit errors and handle fallible calls without panicking.',
  },
  S003: {
    title: 'Unchecked arithmetic',
    severity: 'high',
    summary: 'Overflow, underflow, or truncation can silently distort balances or limits.',
    remediation: 'Use checked arithmetic and return a typed error when an operation cannot be represented safely.',
  },
  S004: {
    title: 'Ledger entry size pressure',
    severity: 'medium',
    summary: 'A write can exceed or approach Soroban ledger entry limits.',
    remediation: 'Split large state, cap collection sizes, or store only stable references on chain.',
  },
  S005: {
    title: 'Storage key collision',
    severity: 'high',
    summary: 'Multiple data paths can write through the same storage key.',
    remediation: 'Namespace keys per feature and include the minimum required discriminators.',
  },
  S006: {
    title: 'Unsafe pattern',
    severity: 'medium',
    summary: 'The contract uses a pattern that is risky in deterministic smart contract execution.',
    remediation: 'Replace timestamp-derived randomness and other unsafe shortcuts with deterministic, auditable flows.',
  },
  S007: {
    title: 'Custom rule match',
    severity: 'medium',
    summary: 'A project-specific rule matched code that the team chose to block or review.',
    remediation: 'Check the custom rule text and either fix the code or document a deliberate suppression.',
  },
  S008: {
    title: 'Event emission issue',
    severity: 'low',
    summary: 'Important state changes may not emit consistent events for wallets and indexers.',
    remediation: 'Emit stable, documented events for externally visible state transitions.',
  },
  S009: {
    title: 'Unhandled Result',
    severity: 'high',
    summary: 'A fallible call returns a Result that is not checked.',
    remediation: 'Match on the Result, propagate failures, and avoid treating failed operations as successful.',
  },
  S010: {
    title: 'Upgrade or governance risk',
    severity: 'high',
    summary: 'Admin, upgrade, or governance paths can create takeover or recovery risk.',
    remediation: 'Use multisig, timelocks, clear authorization checks, and documented emergency paths.',
  },
  S011: {
    title: 'Invariant violation',
    severity: 'critical',
    summary: 'A formal property could not be proven or was disproved by the solver.',
    remediation: 'Review the counterexample, tighten preconditions, or fix the state transition that breaks the invariant.',
  },
  S012: {
    title: 'SEP-41 interface deviation',
    severity: 'medium',
    summary: 'A token contract does not match expected SEP-41 behavior.',
    remediation: 'Align exported functions, auth checks, and error behavior with the SEP-41 interface.',
  },
};

export function normalizeFindingCode(value) {
  const normalized = String(value || '').trim().toUpperCase();
  const match = normalized.match(/S?(\d{1,3})/);

  if (!match) return '';

  return `S${match[1].padStart(3, '0')}`;
}

export function explainFinding(value) {
  const code = normalizeFindingCode(value);
  const finding = FINDINGS[code];

  if (!finding) {
    return {
      found: false,
      code: code || String(value || '').trim(),
      message: 'Unknown finding code. Try one of S001 through S012.',
    };
  }

  return {
    found: true,
    code,
    ...finding,
  };
}

export function formatFindingExplanation(value) {
  const explanation = explainFinding(value);

  if (!explanation.found) {
    return explanation.message;
  }

  return [
    `**${explanation.code}: ${explanation.title}**`,
    `Severity: ${explanation.severity}`,
    explanation.summary,
    `Fix: ${explanation.remediation}`,
  ].join('\n');
}
