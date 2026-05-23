function getLatestUrl() {
  if (process.env.SANCTIFIER_LATEST_URL) {
    return process.env.SANCTIFIER_LATEST_URL;
  }

  if (process.env.SANCTIFIER_API_URL) {
    return new URL('/api/reports/latest', process.env.SANCTIFIER_API_URL).toString();
  }

  return '';
}

function asFindingList(payload) {
  if (Array.isArray(payload)) return payload;
  if (Array.isArray(payload?.findings)) return payload.findings;
  if (Array.isArray(payload?.latest?.findings)) return payload.latest.findings;
  if (Array.isArray(payload?.report?.findings)) return payload.report.findings;

  return [];
}

export async function fetchLatestFindings({ fetchImpl = fetch } = {}) {
  const latestUrl = getLatestUrl();

  if (!latestUrl) {
    return {
      state: 'not_configured',
      findings: [],
      source: '',
    };
  }

  const response = await fetchImpl(latestUrl, {
    headers: { accept: 'application/json' },
  });

  if (!response.ok) {
    return {
      state: 'unavailable',
      findings: [],
      source: latestUrl,
      status: response.status,
    };
  }

  const payload = await response.json();

  return {
    state: 'ok',
    findings: asFindingList(payload),
    source: latestUrl,
  };
}

export function formatLatestFindings(result, limit = 5) {
  if (result.state === 'not_configured') {
    return 'Latest findings are not configured yet. Set SANCTIFIER_LATEST_URL or SANCTIFIER_API_URL for this bot.';
  }

  if (result.state === 'unavailable') {
    return `Could not load latest findings from ${result.source} (HTTP ${result.status}).`;
  }

  if (!result.findings.length) {
    return 'No findings returned by the latest report endpoint.';
  }

  const lines = result.findings.slice(0, limit).map((finding, index) => {
    const code = finding.code || `#${index + 1}`;
    const severity = finding.severity ? ` (${finding.severity})` : '';
    const title = finding.title || finding.message || finding.category || 'Untitled finding';
    const location = finding.location ? ` - ${finding.location}` : '';

    return `${index + 1}. ${code}${severity}: ${title}${location}`;
  });

  return [`Latest Sanctifier findings from ${result.source}:`, ...lines].join('\n');
}
