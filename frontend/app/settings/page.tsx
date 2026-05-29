"use client";

import { useState, useEffect, FormEvent } from "react";

interface Settings {
  analyzerPath: string;
  sanctifyToml: string;
  aiProvider: "stub" | "anthropic" | "openai" | "local";
  aiApiKey: string;
  dailyCostCap: string;
  theme: string;
  telemetryEnabled: boolean;
}

const DEFAULT_SETTINGS: Settings = {
  analyzerPath: "",
  sanctifyToml: "",
  aiProvider: "stub",
  aiApiKey: "",
  dailyCostCap: "10.00",
  theme: "system",
  telemetryEnabled: false,
};

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState<"analyzer" | "ai" | "theme" | "telemetry">("analyzer");
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    // Load settings from localStorage
    try {
      const stored = localStorage.getItem("sanctifier-settings");
      if (stored) {
        setSettings({ ...DEFAULT_SETTINGS, ...JSON.parse(stored) });
      }
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  }, []);

  const handleSave = (e: FormEvent) => {
    e.preventDefault();
    try {
      localStorage.setItem("sanctifier-settings", JSON.stringify(settings));
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (error) {
      console.error("Failed to save settings:", error);
      alert("Failed to save settings. Storage may be full.");
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setSettings({ ...settings, sanctifyToml: content });
    };
    reader.readAsText(file);
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-8">
          Settings
        </h1>

        {/* Tabs */}
        <div className="border-b border-gray-200 dark:border-gray-700 mb-6">
          <nav className="-mb-px flex space-x-8" aria-label="Tabs">
            {[
              { id: "analyzer", label: "Analyzer" },
              { id: "ai", label: "AI" },
              { id: "theme", label: "Theme" },
              { id: "telemetry", label: "Telemetry" },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as typeof activeTab)}
                className={`
                  whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm
                  ${
                    activeTab === tab.id
                      ? "border-blue-500 text-blue-600 dark:text-blue-400"
                      : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                  }
                `}
                aria-current={activeTab === tab.id ? "page" : undefined}
              >
                {tab.label}
              </button>
            ))}
          </nav>
        </div>

        <form onSubmit={handleSave} className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
          {/* Analyzer Tab */}
          {activeTab === "analyzer" && (
            <div className="space-y-6">
              <div>
                <label
                  htmlFor="analyzerPath"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  Binary Path
                </label>
                <input
                  type="text"
                  id="analyzerPath"
                  value={settings.analyzerPath}
                  onChange={(e) =>
                    setSettings({ ...settings, analyzerPath: e.target.value })
                  }
                  placeholder="/usr/local/bin/sanctifier"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                  Path to the Sanctifier binary (SANCTIFIER_BIN)
                </p>
              </div>

              <div>
                <label
                  htmlFor="sanctifyToml"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  .sanctify.toml Override
                </label>
                <textarea
                  id="sanctifyToml"
                  value={settings.sanctifyToml}
                  onChange={(e) =>
                    setSettings({ ...settings, sanctifyToml: e.target.value })
                  }
                  rows={8}
                  placeholder="[rules]&#10;enabled = true"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
              </div>

              <div>
                <label
                  htmlFor="customRules"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  Custom Rules YAML
                </label>
                <input
                  type="file"
                  id="customRules"
                  accept=".yaml,.yml"
                  onChange={handleFileUpload}
                  className="block w-full text-sm text-gray-500 dark:text-gray-400
                    file:mr-4 file:py-2 file:px-4
                    file:rounded-lg file:border-0
                    file:text-sm file:font-medium
                    file:bg-blue-50 file:text-blue-700
                    hover:file:bg-blue-100
                    dark:file:bg-blue-900 dark:file:text-blue-300
                    dark:hover:file:bg-blue-800"
                />
              </div>
            </div>
          )}

          {/* AI Tab */}
          {activeTab === "ai" && (
            <div className="space-y-6">
              <div>
                <label
                  htmlFor="aiProvider"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  AI Provider
                </label>
                <select
                  id="aiProvider"
                  value={settings.aiProvider}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      aiProvider: e.target.value as Settings["aiProvider"],
                    })
                  }
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="stub">Stub (No AI)</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="openai">OpenAI</option>
                  <option value="local">Local Model</option>
                </select>
              </div>

              <div>
                <label
                  htmlFor="aiApiKey"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  API Key
                </label>
                <input
                  type="password"
                  id="aiApiKey"
                  value={settings.aiApiKey}
                  onChange={(e) =>
                    setSettings({ ...settings, aiApiKey: e.target.value })
                  }
                  placeholder="sk-..."
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                  Your API key will be stored locally and never sent to our servers
                </p>
              </div>

              <div>
                <label
                  htmlFor="dailyCostCap"
                  className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                >
                  Daily Cost Cap (USD)
                </label>
                <input
                  type="number"
                  id="dailyCostCap"
                  value={settings.dailyCostCap}
                  onChange={(e) =>
                    setSettings({ ...settings, dailyCostCap: e.target.value })
                  }
                  min="0"
                  step="0.01"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
              </div>
            </div>
          )}

          {/* Theme Tab */}
          {activeTab === "theme" && (
            <div className="space-y-6">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                  Theme Preference
                </label>
                <div className="space-y-2">
                  {[
                    { value: "light", label: "Light" },
                    { value: "dark", label: "Dark" },
                    { value: "system", label: "System" },
                  ].map((option) => (
                    <label
                      key={option.value}
                      className="flex items-center space-x-3 cursor-pointer"
                    >
                      <input
                        type="radio"
                        name="theme"
                        value={option.value}
                        checked={settings.theme === option.value}
                        onChange={(e) =>
                          setSettings({ ...settings, theme: e.target.value })
                        }
                        className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                      />
                      <span className="text-gray-700 dark:text-gray-300">
                        {option.label}
                      </span>
                    </label>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Telemetry Tab */}
          {activeTab === "telemetry" && (
            <div className="space-y-6">
              <div className="flex items-start">
                <div className="flex items-center h-5">
                  <input
                    type="checkbox"
                    id="telemetryEnabled"
                    checked={settings.telemetryEnabled}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        telemetryEnabled: e.target.checked,
                      })
                    }
                    className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                  />
                </div>
                <div className="ml-3">
                  <label
                    htmlFor="telemetryEnabled"
                    className="font-medium text-gray-700 dark:text-gray-300"
                  >
                    Enable Telemetry
                  </label>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Help us improve Sanctifier by sending anonymous usage data
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Save Button */}
          <div className="mt-8 flex items-center justify-between">
            <div>
              {saved && (
                <span className="text-sm text-green-600 dark:text-green-400">
                  ✓ Settings saved successfully
                </span>
              )}
            </div>
            <button
              type="submit"
              className="px-6 py-2.5 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 transition-colors"
            >
              Save Settings
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
