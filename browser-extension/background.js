const DASHBOARD_URL = 'http://localhost:3000';
const API_URL = DASHBOARD_URL + '/api/recent-findings';
const POLL_INTERVAL_MINUTES = 5;

// On install, set up the alarm for periodic polling
chrome.runtime.onInstalled.addListener(function () {
  chrome.alarms.create('pollFindings', {
    periodInMinutes: POLL_INTERVAL_MINUTES,
  });
});

// When the alarm fires, fetch latest findings and cache them
chrome.alarms.onAlarm.addListener(function (alarm) {
  if (alarm.name === 'pollFindings') {
    pollAndCache();
  }
});

// Also poll immediately when the extension starts
pollAndCache();

function pollAndCache() {
  fetch(API_URL)
    .then(function (res) {
      if (!res.ok) throw new Error('HTTP ' + res.status);
      return res.json();
    })
    .then(function (data) {
      // Cache findings in local storage so the popup can access them quickly
      chrome.storage.local.set({
        recentFindings: data,
        lastUpdated: Date.now(),
        status: 'ok',
      });
    })
    .catch(function () {
      // Don't overwrite cached data on error — just mark status
      chrome.storage.local.set({ status: 'error', lastUpdated: Date.now() });
    });
}

// Listen for popup requesting a refresh
chrome.runtime.onConnect.addListener(function (port) {
  if (port.name === 'refreshFindings') {
    pollAndCache();
    port.postMessage({ status: 'refreshing' });
  }
});
