"use client";

import { useRouter } from "next/navigation";
import { PLAYGROUND_SAMPLES } from "../playground/page";
import { Sparkles } from "lucide-react";

export function SamplePicker() {
  const router = useRouter();

  return (
    <div className="flex flex-col items-center gap-3 py-8">
      <div className="flex items-center gap-2 text-sm font-medium text-zinc-500 dark:text-zinc-400">
        <Sparkles className="h-4 w-4" />
        Try a sample
      </div>
      <div className="flex flex-wrap justify-center gap-2">
        {Object.entries(PLAYGROUND_SAMPLES).map(([id, { label }]) => (
          <button
            key={id}
            onClick={() => router.push(`/playground?sample=${id}`)}
            className="rounded-full border border-zinc-300 dark:border-zinc-600 px-3 py-1 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
