const DASHBOARD_URL = "http://localhost:3000";
const API_URL = DASHBOARD_URL + "/api/recent-findings";
const POLL_INTERVAL_MINUTES = 5;
const STALENESS_THRESHOLD_MINUTES = 30; // Show warning after 30 minutes

// On install, set up the alarm for periodic polling
chrome.runtime.onInstalled.addListener(function () {
  chrome.alarms.create("pollFindings", {
    periodInMinutes: POLL_INTERVAL_MINUTES,
  });
  // Also create an alarm to clean up old cached data
  chrome.alarms.create("cleanupCache", {
    periodInMinutes: 60, // Clean up hourly
  });
});

// When the alarm fires, fetch latest findings and cache them
chrome.alarms.onAlarm.addListener(function (alarm) {
  if (alarm.name === "pollFindings") {
    pollAndCache();
  } else if (alarm.name === "cleanupCache") {
    cleanupOldCache();
  }
});

// Also poll immediately when the extension starts
pollAndCache();

function pollAndCache() {
  fetch(API_URL)
    .then(function (res) {
      if (!res.ok) throw new Error("HTTP " + res.status);
      return res.json();
    })
    .then(function (data) {
      // Cache findings in local storage so the popup can access them quickly
      chrome.storage.local.set({
        recentFindings: data,
        lastUpdated: Date.now(),
        status: "ok",
      });
    })
    .catch(function () {
      // Don't overwrite cached data on error — just mark status
      chrome.storage.local.set({ status: "error", lastUpdated: Date.now() });
    });
}

// Listen for popup requesting a refresh
chrome.runtime.onConnect.addListener(function (port) {
  if (port.name === "refreshFindings") {
    pollAndCache();
    port.postMessage({ status: "refreshing" });
  }
});

// Clean up stale cached findings (older than 7 days)
function cleanupOldCache() {
  const MAX_CACHE_AGE_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
  chrome.storage.local.get(["lastUpdated"], function (result) {
    if (result.lastUpdated) {
      const age = Date.now() - result.lastUpdated;
      if (age > MAX_CACHE_AGE_MS) {
        // Clear stale data
        chrome.storage.local.remove([
          "recentFindings",
          "lastUpdated",
          "status",
        ]);
      }
    }
  });
}
