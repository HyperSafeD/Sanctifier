function getStatusUrl() {
  if (process.env.SANCTIFIER_STATUS_URL) {
    return process.env.SANCTIFIER_STATUS_URL;
  }

  if (process.env.SANCTIFIER_API_URL) {
    return process.env.SANCTIFIER_API_URL;
  }

  return '';
}

export async function fetchStatus({ fetchImpl = fetch } = {}) {
  const statusUrl = getStatusUrl();

  if (!statusUrl) {
    return {
      state: 'not_configured',
      source: '',
    };
  }

  try {
    const response = await fetchImpl(statusUrl, {
      method: 'HEAD',
      cache: 'no-store',
    });

    return {
      state: response.ok ? 'online' : 'degraded',
      source: statusUrl,
      status: response.status,
    };
  } catch (error) {
    return {
      state: 'offline',
      source: statusUrl,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export function formatStatus(result) {
  if (result.state === 'not_configured') {
    return 'Sanctifier status is not configured yet. Set SANCTIFIER_API_URL or SANCTIFIER_STATUS_URL.';
  }

  if (result.state === 'online') {
    return `Sanctifier is reachable at ${result.source}.`;
  }

  if (result.state === 'degraded') {
    return `Sanctifier responded from ${result.source}, but with HTTP ${result.status}.`;
  }

  return `Sanctifier is not reachable at ${result.source}: ${result.error}`;
}
