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
