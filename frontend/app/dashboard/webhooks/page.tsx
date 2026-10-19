"use client";

import { useState } from "react";
import { Bell, ChevronRight, Hash, MessageSquare, Save } from "lucide-react";
import Link from "next/link";

export default function WebhooksPage() {
  const [slackUrl, setSlackUrl] = useState("");
  const [discordUrl, setDiscordUrl] = useState("");
  const [filters, setFilters] = useState({
    critical: true,
    high: true,
    medium: false,
    low: false,
    scanCompleted: true,
  });
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = () => {
    setIsSaving(true);
    setTimeout(() => {
      setIsSaving(false);
      alert("Webhook settings saved successfully!");
    }, 1000);
  };

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      <main className="max-w-4xl mx-auto px-4 sm:px-6 py-12 space-y-8">
        {/* Breadcrumbs */}
        <nav className="flex items-center gap-2 text-sm text-zinc-500">
          <Link href="/dashboard" className="hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors">Dashboard</Link>
          <ChevronRight size={14} />
          <span className="text-zinc-900 dark:text-zinc-100 font-medium">Webhooks</span>
        </nav>

        <div className="space-y-2">
          <h1 className="text-3xl font-bold tracking-tight">Integrations & Webhooks</h1>
          <p className="text-zinc-500">Configure real-time notifications for your security scans and team alerts.</p>
        </div>

        <div className="grid grid-cols-1 gap-8">
          {/* Slack Integration */}
          <section className="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm space-y-6">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-orange-500/10 text-orange-500 flex items-center justify-center">
                <Hash size={24} />
              </div>
              <div>
                <h2 className="text-xl font-bold">Slack Integration</h2>
                <p className="text-sm text-zinc-500">Receive vulnerability alerts in your Slack channels.</p>
              </div>
            </div>
            
            <div className="space-y-4">
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-zinc-500">Webhook URL</label>
                <input
                  type="url"
                  value={slackUrl}
                  onChange={(e) => setSlackUrl(e.target.value)}
                  placeholder="https://hooks.slack.com/services/..."
                  className="w-full p-3 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950 outline-none focus:ring-2 focus:ring-orange-500/20 transition-all font-mono text-sm"
                />
              </div>
            </div>
          </section>

          {/* Discord Integration */}
          <section className="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm space-y-6">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-indigo-500/10 text-indigo-500 flex items-center justify-center">
                <MessageSquare size={24} />
              </div>
              <div>
                <h2 className="text-xl font-bold">Discord Integration</h2>
                <p className="text-sm text-zinc-500">Send scan reports directly to your Discord server.</p>
              </div>
            </div>
            
            <div className="space-y-4">
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-zinc-500">Webhook URL</label>
                <input
                  type="url"
                  value={discordUrl}
                  onChange={(e) => setDiscordUrl(e.target.value)}
                  placeholder="https://discord.com/api/webhooks/..."
                  className="w-full p-3 rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950 outline-none focus:ring-2 focus:ring-indigo-500/20 transition-all font-mono text-sm"
                />
              </div>
            </div>
          </section>

          {/* Event Filters */}
          <section className="p-8 rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-sm space-y-6">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-emerald-500/10 text-emerald-500 flex items-center justify-center">
                <Bell size={24} />
              </div>
              <div>
                <h2 className="text-xl font-bold">Notification Filters</h2>
                <p className="text-sm text-zinc-500">Choose which events should trigger a webhook notification.</p>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {Object.entries(filters).map(([key, value]) => (
                <label key={key} className="flex items-center gap-3 p-4 rounded-xl border border-zinc-100 dark:border-zinc-800 hover:bg-zinc-50 dark:hover:bg-zinc-950 transition-colors cursor-pointer">
                  <input
                    type="checkbox"
                    checked={value}
                    onChange={() => setFilters(f => ({ ...f, [key]: !value }))}
                    className="w-4 h-4 rounded border-zinc-300 text-emerald-500 focus:ring-emerald-500"
                  />
                  <span className="text-sm font-medium capitalize">
                    {key.replace(/([A-Z])/g, ' $1').trim()}
                  </span>
                </label>
              ))}
            </div>
          </section>

          <footer className="pt-4 flex justify-end">
            <button
              onClick={handleSave}
              disabled={isSaving}
              className="flex items-center gap-2 px-8 py-3 rounded-xl bg-zinc-900 dark:bg-white text-white dark:text-zinc-900 font-bold hover:scale-[1.02] transition-all shadow-xl active:scale-95 disabled:opacity-50"
            >
              {isSaving ? "Saving Settings..." : (
                <>
                  <Save size={18} />
                  Save Webhook Config
                </>
              )}
            </button>
          </footer>
        </div>
      </main>
    </div>
  );
}
