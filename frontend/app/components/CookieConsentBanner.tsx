"use client";

import { useState, useEffect } from "react";

/**
 * GDPR/ePrivacy-compliant cookie consent banner
 * 
 * This component is currently NOT rendered in the app because no
 * non-essential cookies or tracking scripts are used.
 * 
 * **When to enable**: If analytics/error tracking (like Sentry) is added,
 * import this component in `app/layout.tsx` and render it:
 * 
 * ```tsx
 * import { CookieConsentBanner } from './components/CookieConsentBanner';
 * 
 * export default function RootLayout() {
 *   return (
 *     <html>
 *       <body>
 *         {children}
 *         <CookieConsentBanner />
 *       </body>
 *     </html>
 *   );
 * }
 * ```
 * 
 * **Gating analytics initialization**:
 * ```tsx
 * import { hasConsent } from './components/CookieConsentBanner';
 * 
 * if (hasConsent('analytics')) {
 *   Sentry.init({ ... });
 * }
 * ```
 * 
 * @see frontend/docs/COOKIE_AUDIT.md for compliance details
 * @see #1164 for implementation rationale
 */

const CONSENT_STORAGE_KEY = "sanctifier-cookie-consent";

type ConsentPreferences = {
  necessary: boolean; // Always true
  analytics: boolean;
  timestamp: string;
};

const defaultPreferences: ConsentPreferences = {
  necessary: true,
  analytics: false,
  timestamp: new Date().toISOString(),
};

/**
 * Check if user has given consent for a specific category
 * @param category - 'necessary' | 'analytics'
 * @returns boolean
 */
export function hasConsent(category: keyof ConsentPreferences): boolean {
  if (typeof window === "undefined") return false;
  if (category === "necessary") return true; // Always allowed

  try {
    const stored = localStorage.getItem(CONSENT_STORAGE_KEY);
    if (!stored) return false;

    const prefs: ConsentPreferences = JSON.parse(stored);
    return prefs[category] || false;
  } catch {
    return false;
  }
}

/**
 * Save user consent preferences
 */
function saveConsent(prefs: ConsentPreferences): void {
  try {
    localStorage.setItem(CONSENT_STORAGE_KEY, JSON.stringify(prefs));
  } catch (error) {
    console.error("Failed to save cookie consent:", error);
  }
}

/**
 * Get saved consent preferences
 */
function getConsent(): ConsentPreferences | null {
  if (typeof window === "undefined") return null;

  try {
    const stored = localStorage.getItem(CONSENT_STORAGE_KEY);
    if (!stored) return null;

    return JSON.parse(stored);
  } catch {
    return null;
  }
}

export function CookieConsentBanner() {
  const [showBanner, setShowBanner] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [preferences, setPreferences] = useState<ConsentPreferences>(defaultPreferences);

  useEffect(() => {
    const saved = getConsent();
    if (saved) {
      setPreferences(saved);
      setShowBanner(false);
    } else {
      setShowBanner(true);
    }
  }, []);

  const handleAcceptAll = () => {
    const prefs: ConsentPreferences = {
      necessary: true,
      analytics: true,
      timestamp: new Date().toISOString(),
    };
    saveConsent(prefs);
    setPreferences(prefs);
    setShowBanner(false);
    window.location.reload(); // Reload to initialize analytics
  };

  const handleRejectAll = () => {
    const prefs: ConsentPreferences = {
      necessary: true,
      analytics: false,
      timestamp: new Date().toISOString(),
    };
    saveConsent(prefs);
    setPreferences(prefs);
    setShowBanner(false);
  };

  const handleSavePreferences = () => {
    const prefs: ConsentPreferences = {
      ...preferences,
      timestamp: new Date().toISOString(),
    };
    saveConsent(prefs);
    setShowBanner(false);
    setShowSettings(false);
    window.location.reload(); // Reload to apply new preferences
  };

  const handleToggleAnalytics = () => {
    setPreferences((prev) => ({
      ...prev,
      analytics: !prev.analytics,
    }));
  };

  if (!showBanner) return null;

  return (
    <>
      {/* Banner Overlay */}
      <div
        className="fixed inset-0 bg-black/50 z-40"
        aria-hidden="true"
        onClick={() => setShowSettings(false)}
      />

      {/* Banner Content */}
      <div
        className="fixed bottom-0 left-0 right-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 shadow-lg z-50 p-6"
        role="dialog"
        aria-labelledby="cookie-banner-title"
        aria-describedby="cookie-banner-description"
      >
        <div className="max-w-7xl mx-auto">
          {!showSettings ? (
            // Simple Banner View
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
              <div className="flex-1">
                <h2
                  id="cookie-banner-title"
                  className="text-lg font-semibold text-gray-900 dark:text-white mb-2"
                >
                  🍪 Cookie Preferences
                </h2>
                <p
                  id="cookie-banner-description"
                  className="text-sm text-gray-600 dark:text-gray-300"
                >
                  We use cookies to improve your experience and analyze usage. You can choose
                  which cookies to accept. See our{" "}
                  <a
                    href="/privacy"
                    className="underline hover:text-blue-600 dark:hover:text-blue-400"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Privacy Policy
                  </a>{" "}
                  for more details.
                </p>
              </div>

              <div className="flex flex-wrap gap-3">
                <button
                  onClick={() => setShowSettings(true)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 transition"
                  aria-label="Customize cookie preferences"
                >
                  Customize
                </button>
                <button
                  onClick={handleRejectAll}
                  className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 transition"
                  aria-label="Reject all non-essential cookies"
                >
                  Reject All
                </button>
                <button
                  onClick={handleAcceptAll}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition"
                  aria-label="Accept all cookies"
                >
                  Accept All
                </button>
              </div>
            </div>
          ) : (
            // Settings View
            <div className="space-y-6">
              <div>
                <h2
                  id="cookie-settings-title"
                  className="text-lg font-semibold text-gray-900 dark:text-white mb-2"
                >
                  Cookie Preferences
                </h2>
                <p className="text-sm text-gray-600 dark:text-gray-300">
                  Manage your cookie preferences below. Essential cookies cannot be disabled as
                  they are required for the site to function.
                </p>
              </div>

              <div className="space-y-4">
                {/* Necessary Cookies */}
                <div className="flex items-start justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-gray-50 dark:bg-gray-800">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
                        Essential Cookies
                      </h3>
                      <span className="text-xs text-gray-500 dark:text-gray-400 font-medium">
                        Always Active
                      </span>
                    </div>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Required for basic site functionality like theme preferences and session
                      management. Cannot be disabled.
                    </p>
                  </div>
                  <div className="ml-4">
                    <input
                      type="checkbox"
                      checked={true}
                      disabled
                      className="w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 disabled:opacity-50"
                      aria-label="Essential cookies (always active)"
                    />
                  </div>
                </div>

                {/* Analytics Cookies */}
                <div className="flex items-start justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-gray-300 dark:hover:border-gray-600 transition">
                  <div className="flex-1">
                    <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-1">
                      Analytics & Performance
                    </h3>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Help us understand how visitors use the site so we can improve performance
                      and fix bugs (Sentry error tracking, usage analytics).
                    </p>
                  </div>
                  <div className="ml-4">
                    <input
                      type="checkbox"
                      checked={preferences.analytics}
                      onChange={handleToggleAnalytics}
                      className="w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer"
                      aria-label="Toggle analytics cookies"
                    />
                  </div>
                </div>
              </div>

              <div className="flex justify-end gap-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                <button
                  onClick={() => setShowSettings(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition"
                  aria-label="Cancel and close settings"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSavePreferences}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition"
                  aria-label="Save cookie preferences"
                >
                  Save Preferences
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

/**
 * Optional: Preference center link component for footer
 * Allows users to change their consent after initial decision
 */
export function CookiePreferenceLink() {
  const handleClick = () => {
    localStorage.removeItem(CONSENT_STORAGE_KEY);
    window.location.reload();
  };

  return (
    <button
      onClick={handleClick}
      className="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white underline"
      aria-label="Change cookie preferences"
    >
      Cookie Preferences
    </button>
  );
}
