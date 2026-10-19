/**
 * Browser extension popup (#1893).
 *
 * Renders the last 10 findings fetched by background.js. Handles empty findings
 * and malformed finding shapes gracefully without breaking the popup.
 */

/**
 * Renders findings into the popup DOM.
 *
 * @param {Array} findings - Array of finding objects from background.js.
 */
function renderFindings(findings) {
  const container = document.getElementById('findings-container');
  if (!container) return;

  // Clear existing content
  container.innerHTML = '';

  if (!Array.isArray(findings) || findings.length === 0) {
    container.innerHTML = '<p class="empty-state">No findings available.</p>';
    return;
  }

  const list = document.createElement('ul');
  list.className = 'findings-list';

  findings.slice(0, 10).forEach((finding, index) => {
    const item = document.createElement('li');
    item.className = 'finding-item';

    // Safely extract fields with fallbacks for malformed data
    const severity = finding.severity || 'unknown';
    const code = finding.code || 'N/A';
    const message = finding.message || 'No description available';
    const location = finding.location || 'Unknown location';

    item.innerHTML = `
      <div class="finding-header">
        <span class="severity severity-${severity.toLowerCase()}">${severity.toUpperCase()}</span>
        <span class="code">${code}</span>
      </div>
      <div class="finding-body">
        <p class="message">${escapeHtml(message)}</p>
        <p class="location">${escapeHtml(location)}</p>
      </div>
    `;

    list.appendChild(item);
  });

  container.appendChild(list);
}

/**
 * Escapes HTML to prevent XSS when rendering user-supplied data.
 *
 * @param {string} str - String to escape.
 * @returns {string} HTML-escaped string.
 */
function escapeHtml(str) {
  if (typeof str !== 'string') return String(str);
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Export for testing (Node.js environment)
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { renderFindings, escapeHtml };
}

// Browser environment: fetch findings from storage and render
if (typeof chrome !== 'undefined' && chrome.storage) {
  chrome.storage.local.get(['recentFindings'], (result) => {
    renderFindings(result.recentFindings || []);
  });
}
(function () {
  'use strict';

  const DASHBOARD_URL = 'http://localhost:3000';
  const API_URL = DASHBOARD_URL + '/api/recent-findings';

  const badgeEl = document.getElementById('badge');
  const statusDot = document.getElementById('statusDot');
  const statusText = document.getElementById('statusText');
  const contentEl = document.getElementById('content');

  function setStatus(state, text) {
    statusDot.className = 'dot ' + state;
    statusText.textContent = text;
  }

  function setBadge(state, text) {
    badgeEl.className = 'badge ' + state;
    badgeEl.textContent = text;
  }

  function severityClass(severity) {
    return 'severity-' + (severity || 'low');
  }

  function escHtml(str) {
    if (!str) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function renderFindings(data) {
    const findings = data.findings || [];
    const count = data.count || 0;
    const total = data.total_available || 0;

    if (findings.length === 0) {
      contentEl.innerHTML =
        '<div class="empty-state">' +
        '<div class="icon">🔍</div>' +
        '<p>No recent findings.</p>' +
        '<p style="margin-top:4px;font-size:12px;">Run a scan in the dashboard.</p>' +
        '</div>';
      setBadge('ok', 'No findings');
      return;
    }

    let html = '';
    for (const f of findings) {
      const statusClass = (f.status === 'added' || f.status === 'regression') ? ' finding added' : '';
      html +=
        '<div class="finding' + statusClass + '" data-id="' + escHtml(f.id) + '">' +
        '<div class="finding-header">' +
        '<span class="finding-code">' + escHtml(f.code) + '</span>' +
        '<span class="finding-severity ' + severityClass(f.severity) + '">' +
        escHtml(f.severity) +
        '</span>' +
        '</div>' +
        '<div class="finding-title">' + escHtml(f.title) + '</div>' +
        '<div class="finding-location">' + escHtml(f.location) + '</div>' +
        '</div>';
    }

    contentEl.innerHTML = html;

    // Attach click handlers for deep linking
    contentEl.querySelectorAll('.finding').forEach(function (el) {
      el.addEventListener('click', function () {
        chrome.tabs.create({ url: DASHBOARD_URL + '/dashboard' });
      });
    });

    setBadge('ok', count + ' finding' + (count !== 1 ? 's' : ''));
    if (count < total) {
      setBadge('ok', count + ' of ' + total);
    }
  }

  function renderError(msg) {
    contentEl.innerHTML =
      '<div class="empty-state">' +
      '<div class="icon">⚠️</div>' +
      '<p>' + escHtml(msg) + '</p>' +
      '<p style="margin-top:4px;font-size:12px;">Ensure the Sanctifier dashboard is running at ' +
      escHtml(DASHBOARD_URL) + '.</p>' +
      '</div>';
    setBadge('error', 'Error');
  }

  function fetchFindings() {
    setStatus('loading', 'Fetching findings…');

    fetch(API_URL)
      .then(function (res) {
        if (!res.ok) {
          throw new Error('HTTP ' + res.status + ': ' + res.statusText);
        }
        return res.json();
      })
      .then(function (data) {
        if (data.status === 'ok') {
          setStatus('connected', 'Connected · Last ' + (data.count || 0) + ' findings');
          renderFindings(data);
        } else if (data.status === 'no_data') {
          renderFindings({ findings: [], count: 0, total_available: 0 });
        } else {
          renderError('Unexpected response from server.');
        }
      })
      .catch(function (err) {
        setStatus('disconnected', 'Disconnected');
        renderError('Could not connect: ' + err.message);
      });
  }

  // Initial fetch on popup open
  fetchFindings();

  // Refresh every 30 seconds while popup is open
  var refreshInterval = setInterval(fetchFindings, 30000);

  // Clean up interval when popup closes
  window.addEventListener('unload', function () {
    clearInterval(refreshInterval);
  });
})();
