const DASHBOARD_URL = "http://localhost:3000";
const API_URL = DASHBOARD_URL + "/api/recent-findings";
const POLL_INTERVAL_MINUTES = 5;

// On install, set up the alarm for periodic polling
chrome.runtime.onInstalled.addListener(function () {
  chrome.alarms.create("pollFindings", {
    periodInMinutes: POLL_INTERVAL_MINUTES,
  });
});

// When the alarm fires, fetch latest findings and cache them
chrome.alarms.onAlarm.addListener(function (alarm) {
  if (alarm.name === "pollFindings") {
    pollAndCache();
  }
});

// Also poll immediately when the extension starts
pollAndCache();

function pollAndCache() {
  fetch(API_URL)
    .then(function (res) {
      if (!res.ok) {
        throw new Error("HTTP " + res.status + ": " + res.statusText);
      }
      // Check content type to ensure we're getting JSON
      const contentType = res.headers.get("content-type");
      if (!contentType || !contentType.includes("application/json")) {
        throw new Error(
          "Non-JSON response (content-type: " + (contentType || "none") + ")",
        );
      }
      return res.json();
    })
    .then(function (data) {
      // Validate that data is a valid object (not null, undefined, or malformed)
      if (!data || typeof data !== "object") {
        throw new Error("Invalid JSON structure");
      }
      // Cache findings in local storage so the popup can access them quickly
      chrome.storage.local.set({
        recentFindings: data,
        lastUpdated: Date.now(),
        status: "ok",
        errorMessage: null,
      });
    })
    .catch(function (err) {
      // Don't overwrite cached data on error — just mark status and store error details
      chrome.storage.local.set({
        status: "error",
        lastUpdated: Date.now(),
        errorMessage: err.message || "Unknown error",
      });
      console.error("Sanctifier dashboard poll failed:", err);
    });
}

// Listen for popup requesting a refresh
chrome.runtime.onConnect.addListener(function (port) {
  if (port.name === "refreshFindings") {
    pollAndCache();
    port.postMessage({ status: "refreshing" });
  }
});
